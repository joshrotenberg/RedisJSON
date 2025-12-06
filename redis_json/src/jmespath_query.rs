/*
 * Copyright (c) 2006-Present, Redis Ltd.
 * All rights reserved.
 *
 * Licensed under your choice of (a) the Redis Source Available License 2.0
 * (RSALv2); or (b) the Server Side Public License v1 (SSPLv1); or (c) the
 * GNU Affero General Public License v3 (AGPLv3).
 */

//! JMESPath query support for RedisJSON
//!
//! This module provides JMESPath expression compilation and evaluation.
//! JMESPath is a query language for JSON that supports projections,
//! filters, pipes, and transformations.
//!
//! Unlike JSONPath, JMESPath is read-only and focused on data extraction
//! and transformation rather than mutation.
//!
//! ## Expression Caching
//!
//! Compiled JMESPath expressions are cached per-thread to avoid repeated parsing.
//! The cache uses an LRU eviction policy with a configurable maximum size.
//! Thread-local caching is used because the jmespath crate's Expression type
//! is not Send (it contains Rc internally).

use std::cell::RefCell;
use std::collections::HashMap;

use jmespath::{Expression, Rcvar, ToJmespath, Variable};

use crate::jmespath_functions::REDIS_RUNTIME;
use redis_module::redisvalue::{RedisValue, RedisValueKey};
use serde::Serialize;

use crate::error::Error;
use crate::formatter::{RedisJsonFormatter, ReplyFormatOptions};

/// Maximum number of compiled expressions to cache per thread.
/// This provides a reasonable balance between memory usage and cache hit rate.
const EXPRESSION_CACHE_SIZE: usize = 256;

/// Entry in the expression cache, tracking usage for LRU eviction.
struct CacheEntry {
    expression: Expression<'static>,
    last_used: u64,
}

/// Thread-local LRU cache for compiled JMESPath expressions.
struct ExpressionCache {
    entries: HashMap<String, CacheEntry>,
    counter: u64,
}

impl ExpressionCache {
    fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(EXPRESSION_CACHE_SIZE),
            counter: 0,
        }
    }

    /// Get or compile an expression, updating LRU tracking.
    fn get_or_compile(&mut self, expr: &str) -> Result<&Expression<'static>, Error> {
        self.counter += 1;
        let current_counter = self.counter;

        // Check if already cached
        if self.entries.contains_key(expr) {
            let entry = self.entries.get_mut(expr).unwrap();
            entry.last_used = current_counter;
            return Ok(&self.entries.get(expr).unwrap().expression);
        }

        // Compile the new expression using custom Redis runtime
        let compiled = REDIS_RUNTIME
            .compile(expr)
            .map_err(|e| Error::from(format!("ERR JMESPath compile error: {e}")))?;

        // Evict least recently used if at capacity
        if self.entries.len() >= EXPRESSION_CACHE_SIZE {
            if let Some(lru_key) = self
                .entries
                .iter()
                .min_by_key(|(_, v)| v.last_used)
                .map(|(k, _)| k.clone())
            {
                self.entries.remove(&lru_key);
            }
        }

        // Insert the new entry
        self.entries.insert(
            expr.to_string(),
            CacheEntry {
                expression: compiled,
                last_used: current_counter,
            },
        );

        Ok(&self.entries.get(expr).unwrap().expression)
    }
}

thread_local! {
    static EXPRESSION_CACHE: RefCell<ExpressionCache> = RefCell::new(ExpressionCache::new());
}

/// Compile and evaluate a JMESPath expression against a value.
///
/// The value must implement `Serialize` (which IValue does), allowing
/// the jmespath crate's blanket `ToJmespath` impl to handle conversion.
///
/// Compiled expressions are cached per-thread for performance. The cache
/// uses LRU eviction when it reaches capacity.
///
/// Returns the result serialized as a JSON string with the given format options.
pub fn query<V: Serialize + ToJmespath>(
    expr: &str,
    value: &V,
    format: &ReplyFormatOptions,
) -> Result<String, Error> {
    // Get or compile expression (with thread-local caching)
    let result = EXPRESSION_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let expression = cache.get_or_compile(expr)?;

        // Evaluate - ToJmespath blanket impl handles Serialize types
        expression
            .search(value)
            .map_err(|e| Error::from(format!("ERR JMESPath error: {e}")))
    })?;

    // Serialize result with formatting options
    Ok(serialize_rcvar(&result, format))
}

/// Serialize an Rcvar (JMESPath result) to a JSON string with formatting options.
fn serialize_rcvar(value: &Rcvar, format: &ReplyFormatOptions) -> String {
    if format.no_formatting() {
        serde_json::to_string(&**value).unwrap_or_else(|_| "null".to_string())
    } else {
        let formatter = RedisJsonFormatter::new(format);
        let mut out = serde_json::Serializer::with_formatter(Vec::new(), formatter);
        if serde::Serialize::serialize(&**value, &mut out).is_ok() {
            String::from_utf8(out.into_inner()).unwrap_or_else(|_| "null".to_string())
        } else {
            "null".to_string()
        }
    }
}

/// Compile and evaluate a JMESPath expression, returning RESP3 native types.
///
/// This function is used when the client requests FORMAT EXPAND with RESP3.
/// The JMESPath result is converted directly to Redis native types:
/// - null -> RedisValue::Null
/// - bool -> RedisValue::Bool
/// - number -> RedisValue::Integer or RedisValue::Float
/// - string -> RedisValue::BulkString
/// - array -> RedisValue::Array
/// - object -> RedisValue::Map
pub fn query_to_resp3<V: Serialize + ToJmespath>(
    expr: &str,
    value: &V,
) -> Result<RedisValue, Error> {
    // Get or compile expression (with thread-local caching)
    let result = EXPRESSION_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let expression = cache.get_or_compile(expr)?;

        // Evaluate - ToJmespath blanket impl handles Serialize types
        expression
            .search(value)
            .map_err(|e| Error::from(format!("ERR JMESPath error: {e}")))
    })?;

    // Convert to RESP3 native types
    Ok(rcvar_to_redis_value(&result))
}

/// Convert a JMESPath Variable to a Redis native value for RESP3 output.
fn rcvar_to_redis_value(value: &Rcvar) -> RedisValue {
    match &**value {
        Variable::Null => RedisValue::Null,
        Variable::Bool(b) => RedisValue::Bool(*b),
        Variable::Number(n) => {
            // Try to represent as integer if possible, otherwise use float
            if let Some(i) = n.as_i64() {
                RedisValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                RedisValue::Float(f)
            } else {
                // Fallback - shouldn't happen with valid JSON numbers
                RedisValue::Null
            }
        }
        Variable::String(s) => RedisValue::BulkString(s.clone()),
        Variable::Array(arr) => RedisValue::Array(arr.iter().map(rcvar_to_redis_value).collect()),
        Variable::Object(obj) => RedisValue::Map(
            obj.iter()
                .map(|(k, v)| (RedisValueKey::String(k.clone()), rcvar_to_redis_value(v)))
                .collect(),
        ),
        Variable::Expref(_) => {
            // Expression references shouldn't appear in output, serialize as string
            RedisValue::BulkString("<expression>".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn default_format() -> ReplyFormatOptions<'static> {
        ReplyFormatOptions::default()
    }

    // =========================================================================
    // Basic JMESPath expressions
    // =========================================================================

    #[test]
    fn test_basic_field_access() {
        let data = json!({"name": "Alice", "age": 30});
        let result = query("name", &data, &default_format()).unwrap();
        assert_eq!(result, r#""Alice""#);
    }

    #[test]
    fn test_nested_field_access() {
        let data = json!({"user": {"name": "Alice", "age": 30}});
        let result = query("user.name", &data, &default_format()).unwrap();
        assert_eq!(result, r#""Alice""#);
    }

    #[test]
    fn test_array_index() {
        let data = json!({"items": ["a", "b", "c"]});
        let result = query("items[1]", &data, &default_format()).unwrap();
        assert_eq!(result, r#""b""#);
    }

    #[test]
    fn test_negative_array_index() {
        let data = json!({"items": ["a", "b", "c"]});
        let result = query("items[-1]", &data, &default_format()).unwrap();
        assert_eq!(result, r#""c""#);
    }

    #[test]
    fn test_projection() {
        let data = json!({"people": [{"name": "Alice"}, {"name": "Bob"}]});
        let result = query("people[*].name", &data, &default_format()).unwrap();
        assert_eq!(result, r#"["Alice","Bob"]"#);
    }

    #[test]
    fn test_filter() {
        let data = json!({"items": [{"price": 10}, {"price": 25}, {"price": 5}]});
        let result = query("items[?price > `10`].price", &data, &default_format()).unwrap();
        assert_eq!(result, "[25]");
    }

    #[test]
    fn test_pipe_expression() {
        let data = json!({"names": ["charlie", "alice", "bob"]});
        let result = query("names | sort(@)", &data, &default_format()).unwrap();
        assert_eq!(result, r#"["alice","bob","charlie"]"#);
    }

    #[test]
    fn test_multiselect_hash() {
        let data = json!({"person": {"firstName": "Alice", "age": 30}});
        let result = query(
            "person.{name: firstName, years: age}",
            &data,
            &default_format(),
        )
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["years"], 30);
    }

    #[test]
    fn test_multiselect_list() {
        let data = json!({"a": 1, "b": 2, "c": 3});
        let result = query("[a, b, c]", &data, &default_format()).unwrap();
        assert_eq!(result, "[1,2,3]");
    }

    #[test]
    fn test_flatten() {
        let data = json!({"arrays": [[1, 2], [3, 4]]});
        let result = query("arrays[]", &data, &default_format()).unwrap();
        assert_eq!(result, "[1,2,3,4]");
    }

    #[test]
    fn test_slice() {
        let data = json!({"items": [0, 1, 2, 3, 4, 5]});
        let result = query("items[1:4]", &data, &default_format()).unwrap();
        assert_eq!(result, "[1,2,3]");
    }

    #[test]
    fn test_missing_field_returns_null() {
        let data = json!({"foo": "bar"});
        let result = query("missing", &data, &default_format()).unwrap();
        assert_eq!(result, "null");
    }

    #[test]
    fn test_empty_filter_result() {
        let data = json!({"items": [{"x": 1}, {"x": 2}]});
        let result = query("items[?x > `100`]", &data, &default_format()).unwrap();
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_invalid_expression() {
        let data = json!({});
        let result = query("[invalid", &data, &default_format());
        assert!(result.is_err());
        assert!(result.unwrap_err().msg.contains("JMESPath compile error"));
    }

    // =========================================================================
    // Built-in functions: Math
    // =========================================================================

    #[test]
    fn test_fn_abs() {
        let data = json!({"n": -5});
        let result = query("abs(n)", &data, &default_format()).unwrap();
        assert_eq!(result, "5.0");

        let data = json!({"n": 3.14});
        let result = query("abs(n)", &data, &default_format()).unwrap();
        assert_eq!(result, "3.14");
    }

    #[test]
    fn test_fn_avg() {
        let data = json!({"nums": [1, 2, 3, 4, 5]});
        let result = query("avg(nums)", &data, &default_format()).unwrap();
        assert_eq!(result, "3.0");

        // Single element
        let data = json!({"nums": [10]});
        let result = query("avg(nums)", &data, &default_format()).unwrap();
        assert_eq!(result, "10.0");
    }

    #[test]
    fn test_fn_ceil() {
        let data = json!({"n": 1.2});
        let result = query("ceil(n)", &data, &default_format()).unwrap();
        assert_eq!(result, "2.0");

        let data = json!({"n": -1.8});
        let result = query("ceil(n)", &data, &default_format()).unwrap();
        assert_eq!(result, "-1.0");
    }

    #[test]
    fn test_fn_floor() {
        let data = json!({"n": 1.8});
        let result = query("floor(n)", &data, &default_format()).unwrap();
        assert_eq!(result, "1.0");

        let data = json!({"n": -1.2});
        let result = query("floor(n)", &data, &default_format()).unwrap();
        assert_eq!(result, "-2.0");
    }

    #[test]
    fn test_fn_max() {
        let data = json!({"nums": [3, 1, 4, 1, 5, 9, 2, 6]});
        let result = query("max(nums)", &data, &default_format()).unwrap();
        assert_eq!(result, "9");

        // With strings
        let data = json!({"strs": ["banana", "apple", "cherry"]});
        let result = query("max(strs)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""cherry""#);
    }

    #[test]
    fn test_fn_min() {
        let data = json!({"nums": [3, 1, 4, 1, 5, 9, 2, 6]});
        let result = query("min(nums)", &data, &default_format()).unwrap();
        assert_eq!(result, "1");

        // With strings
        let data = json!({"strs": ["banana", "apple", "cherry"]});
        let result = query("min(strs)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""apple""#);
    }

    #[test]
    fn test_fn_sum() {
        let data = json!({"nums": [1, 2, 3, 4, 5]});
        let result = query("sum(nums)", &data, &default_format()).unwrap();
        assert_eq!(result, "15.0");

        // Empty array
        let data = json!({"nums": []});
        let result = query("sum(nums)", &data, &default_format()).unwrap();
        assert_eq!(result, "0.0");
    }

    // =========================================================================
    // Built-in functions: String
    // =========================================================================

    #[test]
    fn test_fn_contains_array() {
        let data = json!({"items": ["a", "b", "c"]});
        let result = query("contains(items, 'b')", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        let result = query("contains(items, 'z')", &data, &default_format()).unwrap();
        assert_eq!(result, "false");
    }

    #[test]
    fn test_fn_contains_string() {
        let data = json!({"s": "hello world"});
        let result = query("contains(s, 'world')", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        let result = query("contains(s, 'foo')", &data, &default_format()).unwrap();
        assert_eq!(result, "false");
    }

    #[test]
    fn test_fn_ends_with() {
        let data = json!({"s": "hello world"});
        let result = query("ends_with(s, 'world')", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        let result = query("ends_with(s, 'hello')", &data, &default_format()).unwrap();
        assert_eq!(result, "false");
    }

    #[test]
    fn test_fn_starts_with() {
        let data = json!({"s": "hello world"});
        let result = query("starts_with(s, 'hello')", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        let result = query("starts_with(s, 'world')", &data, &default_format()).unwrap();
        assert_eq!(result, "false");
    }

    #[test]
    fn test_fn_join() {
        let data = json!({"items": ["a", "b", "c"]});
        let result = query("join(', ', items)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""a, b, c""#);

        // Empty separator
        let result = query("join('', items)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""abc""#);
    }

    #[test]
    fn test_fn_length_array() {
        let data = json!({"items": [1, 2, 3, 4, 5]});
        let result = query("length(items)", &data, &default_format()).unwrap();
        assert_eq!(result, "5");
    }

    #[test]
    fn test_fn_length_string() {
        let data = json!({"s": "hello"});
        let result = query("length(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "5");
    }

    #[test]
    fn test_fn_length_object() {
        let data = json!({"obj": {"a": 1, "b": 2, "c": 3}});
        let result = query("length(obj)", &data, &default_format()).unwrap();
        assert_eq!(result, "3");
    }

    // =========================================================================
    // Built-in functions: Array/Object
    // =========================================================================

    #[test]
    fn test_fn_keys() {
        let data = json!({"a": 1, "b": 2, "c": 3});
        let result = query("keys(@)", &data, &default_format()).unwrap();
        let parsed: Vec<String> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 3);
        assert!(parsed.contains(&"a".to_string()));
        assert!(parsed.contains(&"b".to_string()));
        assert!(parsed.contains(&"c".to_string()));
    }

    #[test]
    fn test_fn_values() {
        let data = json!({"a": 1, "b": 2, "c": 3});
        let result = query("values(@)", &data, &default_format()).unwrap();
        let parsed: Vec<i64> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 3);
        assert!(parsed.contains(&1));
        assert!(parsed.contains(&2));
        assert!(parsed.contains(&3));
    }

    #[test]
    fn test_fn_merge() {
        let data = json!({"a": {"x": 1}, "b": {"y": 2}});
        let result = query("merge(a, b)", &data, &default_format()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["x"], 1);
        assert_eq!(parsed["y"], 2);
    }

    #[test]
    fn test_fn_merge_override() {
        // Later objects override earlier ones
        let data = json!({"a": {"x": 1}, "b": {"x": 2}});
        let result = query("merge(a, b)", &data, &default_format()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["x"], 2);
    }

    #[test]
    fn test_fn_reverse_array() {
        let data = json!({"items": [1, 2, 3, 4, 5]});
        let result = query("reverse(items)", &data, &default_format()).unwrap();
        assert_eq!(result, "[5,4,3,2,1]");
    }

    #[test]
    fn test_fn_reverse_string() {
        let data = json!({"s": "hello"});
        let result = query("reverse(s)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""olleh""#);
    }

    #[test]
    fn test_fn_sort() {
        let data = json!({"nums": [3, 1, 4, 1, 5, 9, 2, 6]});
        let result = query("sort(nums)", &data, &default_format()).unwrap();
        assert_eq!(result, "[1,1,2,3,4,5,6,9]");

        // With strings
        let data = json!({"strs": ["banana", "apple", "cherry"]});
        let result = query("sort(strs)", &data, &default_format()).unwrap();
        assert_eq!(result, r#"["apple","banana","cherry"]"#);
    }

    #[test]
    fn test_fn_sort_by() {
        let data = json!({"users": [
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25},
            {"name": "Carol", "age": 35}
        ]});
        let result = query("sort_by(users, &age)[*].name", &data, &default_format()).unwrap();
        assert_eq!(result, r#"["Bob","Alice","Carol"]"#);
    }

    #[test]
    fn test_fn_max_by() {
        let data = json!({"users": [
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25},
            {"name": "Carol", "age": 35}
        ]});
        let result = query("max_by(users, &age).name", &data, &default_format()).unwrap();
        assert_eq!(result, r#""Carol""#);
    }

    #[test]
    fn test_fn_min_by() {
        let data = json!({"users": [
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25},
            {"name": "Carol", "age": 35}
        ]});
        let result = query("min_by(users, &age).name", &data, &default_format()).unwrap();
        assert_eq!(result, r#""Bob""#);
    }

    #[test]
    fn test_fn_map() {
        let data = json!({"nums": [1, 2, 3]});
        let result = query("map(&to_string(@), nums)", &data, &default_format()).unwrap();
        assert_eq!(result, r#"["1","2","3"]"#);
    }

    // =========================================================================
    // Built-in functions: Type conversion
    // =========================================================================

    #[test]
    fn test_fn_to_string() {
        let data = json!({"n": 42});
        let result = query("to_string(n)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""42""#);

        let data = json!({"b": true});
        let result = query("to_string(b)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""true""#);
    }

    #[test]
    fn test_fn_to_number() {
        // Integer string - may serialize as "42" or "42.0" depending on library
        let data = json!({"s": "42"});
        let result = query("to_number(s)", &data, &default_format()).unwrap();
        let parsed: f64 = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed, 42.0);

        let data = json!({"s": "3.14"});
        let result = query("to_number(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "3.14");

        // Invalid string returns null
        let data = json!({"s": "not a number"});
        let result = query("to_number(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "null");
    }

    #[test]
    fn test_fn_to_array() {
        let data = json!({"s": "hello"});
        let result = query("to_array(s)", &data, &default_format()).unwrap();
        assert_eq!(result, r#"["hello"]"#);

        // Already an array - returns as-is
        let data = json!({"arr": [1, 2, 3]});
        let result = query("to_array(arr)", &data, &default_format()).unwrap();
        assert_eq!(result, "[1,2,3]");
    }

    #[test]
    fn test_fn_type() {
        let data = json!({"s": "hello", "n": 42, "b": true, "arr": [], "obj": {}, "null": null});

        let result = query("type(s)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""string""#);

        let result = query("type(n)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""number""#);

        let result = query("type(b)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""boolean""#);

        let result = query("type(arr)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""array""#);

        let result = query("type(obj)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""object""#);

        let result = query("type(null)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""null""#);
    }

    #[test]
    fn test_fn_not_null() {
        let data = json!({"a": null, "b": null, "c": "found"});
        let result = query("not_null(a, b, c)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""found""#);

        // All null
        let data = json!({"a": null, "b": null});
        let result = query("not_null(a, b)", &data, &default_format()).unwrap();
        assert_eq!(result, "null");
    }

    // =========================================================================
    // Formatting options
    // =========================================================================

    #[test]
    fn test_formatting_with_indent() {
        let data = json!({"name": "Alice"});
        let format = ReplyFormatOptions {
            indent: Some("  "),
            newline: Some("\n"),
            space: Some(" "),
            ..Default::default()
        };
        let result = query("@", &data, &format).unwrap();
        assert!(result.contains('\n'));
        assert!(result.contains("  "));
    }

    #[test]
    fn test_formatting_compact() {
        let data = json!({"a": 1, "b": 2});
        let result = query("@", &data, &default_format()).unwrap();
        // No extra whitespace
        assert!(!result.contains('\n'));
        assert!(!result.contains("  "));
    }

    // =========================================================================
    // Edge cases
    // =========================================================================

    #[test]
    fn test_deeply_nested() {
        let data = json!({"a": {"b": {"c": {"d": {"e": "deep"}}}}});
        let result = query("a.b.c.d.e", &data, &default_format()).unwrap();
        assert_eq!(result, r#""deep""#);
    }

    #[test]
    fn test_unicode_strings() {
        let data = json!({"greeting": "こんにちは", "emoji": "🎉"});
        let result = query("greeting", &data, &default_format()).unwrap();
        assert_eq!(result, r#""こんにちは""#);

        let result = query("emoji", &data, &default_format()).unwrap();
        assert_eq!(result, r#""🎉""#);
    }

    #[test]
    fn test_large_array() {
        let nums: Vec<i32> = (0..1000).collect();
        let data = json!({"nums": nums});
        let result = query("length(nums)", &data, &default_format()).unwrap();
        assert_eq!(result, "1000");

        let result = query("max(nums)", &data, &default_format()).unwrap();
        assert_eq!(result, "999");
    }

    #[test]
    fn test_boolean_filter_expressions() {
        let data = json!({"items": [
            {"name": "a", "active": true, "count": 5},
            {"name": "b", "active": false, "count": 10},
            {"name": "c", "active": true, "count": 3}
        ]});

        // AND
        let result = query(
            "items[?active && count > `3`].name",
            &data,
            &default_format(),
        )
        .unwrap();
        assert_eq!(result, r#"["a"]"#);

        // OR
        let result = query(
            "items[?!active || count < `4`].name",
            &data,
            &default_format(),
        )
        .unwrap();
        assert_eq!(result, r#"["b","c"]"#);

        // NOT
        let result = query("items[?!active].name", &data, &default_format()).unwrap();
        assert_eq!(result, r#"["b"]"#);
    }

    #[test]
    fn test_current_node_reference() {
        let data = json!([1, 2, 3, 4, 5]);
        let result = query("@ | sum(@)", &data, &default_format()).unwrap();
        assert_eq!(result, "15.0");
    }

    // =========================================================================
    // Additional edge cases
    // =========================================================================

    #[test]
    fn test_empty_object() {
        let data = json!({});
        let result = query("@", &data, &default_format()).unwrap();
        assert_eq!(result, "{}");

        let result = query("keys(@)", &data, &default_format()).unwrap();
        assert_eq!(result, "[]");

        let result = query("values(@)", &data, &default_format()).unwrap();
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_empty_array() {
        let data = json!({"arr": []});
        let result = query("arr", &data, &default_format()).unwrap();
        assert_eq!(result, "[]");

        let result = query("length(arr)", &data, &default_format()).unwrap();
        assert_eq!(result, "0");

        let result = query("arr[0]", &data, &default_format()).unwrap();
        assert_eq!(result, "null");
    }

    #[test]
    fn test_null_values() {
        let data = json!({"a": null, "b": [null, null], "c": {"nested": null}});

        let result = query("a", &data, &default_format()).unwrap();
        assert_eq!(result, "null");

        let result = query("b[0]", &data, &default_format()).unwrap();
        assert_eq!(result, "null");

        let result = query("c.nested", &data, &default_format()).unwrap();
        assert_eq!(result, "null");

        // Filter with null
        let result = query("b[?@ != null]", &data, &default_format()).unwrap();
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_special_characters_in_keys() {
        // Keys with spaces, dots, special chars need bracket notation
        let data = json!({"key with spaces": 1, "key.with.dots": 2, "key-with-dashes": 3});

        let result = query("\"key with spaces\"", &data, &default_format()).unwrap();
        assert_eq!(result, "1");

        let result = query("\"key.with.dots\"", &data, &default_format()).unwrap();
        assert_eq!(result, "2");

        // Dashes work with regular identifier syntax
        let result = query("\"key-with-dashes\"", &data, &default_format()).unwrap();
        assert_eq!(result, "3");
    }

    #[test]
    fn test_numeric_string_keys() {
        let data = json!({"123": "numeric key", "0": "zero key"});

        let result = query("\"123\"", &data, &default_format()).unwrap();
        assert_eq!(result, r#""numeric key""#);

        let result = query("\"0\"", &data, &default_format()).unwrap();
        assert_eq!(result, r#""zero key""#);
    }

    #[test]
    fn test_mixed_type_array() {
        let data = json!({"mixed": [1, "two", true, null, {"nested": "obj"}, [1, 2, 3]]});

        let result = query("mixed[0]", &data, &default_format()).unwrap();
        assert_eq!(result, "1");

        let result = query("mixed[1]", &data, &default_format()).unwrap();
        assert_eq!(result, r#""two""#);

        let result = query("mixed[2]", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        let result = query("mixed[4].nested", &data, &default_format()).unwrap();
        assert_eq!(result, r#""obj""#);

        let result = query("mixed[5][1]", &data, &default_format()).unwrap();
        assert_eq!(result, "2");
    }

    #[test]
    fn test_escaped_strings() {
        let data =
            json!({"quote": "he said \"hello\"", "newline": "line1\nline2", "tab": "col1\tcol2"});

        let result = query("quote", &data, &default_format()).unwrap();
        assert_eq!(result, r#""he said \"hello\"""#);

        let result = query("newline", &data, &default_format()).unwrap();
        assert_eq!(result, "\"line1\\nline2\"");

        let result = query("tab", &data, &default_format()).unwrap();
        assert_eq!(result, "\"col1\\tcol2\"");
    }

    #[test]
    fn test_numeric_precision() {
        let data = json!({
            "int": 9007199254740993_i64,
            "float": 3.141592653589793,
            "scientific": 1.23e10,
            "negative": -999999999999_i64
        });

        let result = query("int", &data, &default_format()).unwrap();
        assert_eq!(result, "9007199254740993");

        let result = query("float", &data, &default_format()).unwrap();
        assert_eq!(result, "3.141592653589793");

        let result = query("scientific", &data, &default_format()).unwrap();
        assert!(result.contains("12300000000") || result.contains("1.23e10"));

        let result = query("negative", &data, &default_format()).unwrap();
        assert_eq!(result, "-999999999999");
    }

    #[test]
    fn test_wildcard_on_object() {
        let data = json!({"a": 1, "b": 2, "c": 3});
        let result = query("*", &data, &default_format()).unwrap();
        let parsed: Vec<i64> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 3);
        assert!(parsed.contains(&1));
        assert!(parsed.contains(&2));
        assert!(parsed.contains(&3));
    }

    #[test]
    fn test_chained_filters() {
        let data = json!({"items": [
            {"category": "A", "price": 10, "stock": 5},
            {"category": "A", "price": 20, "stock": 0},
            {"category": "B", "price": 15, "stock": 3},
            {"category": "A", "price": 5, "stock": 10}
        ]});

        // Filter by category, then by stock > 0, then get prices
        let result = query(
            "items[?category == 'A'] | [?stock > `0`].price",
            &data,
            &default_format(),
        )
        .unwrap();
        assert_eq!(result, "[10,5]");
    }

    #[test]
    fn test_complex_multiselect() {
        let data = json!({
            "order": {
                "id": "ORD-123",
                "items": [
                    {"name": "Widget", "qty": 2, "price": 10},
                    {"name": "Gadget", "qty": 1, "price": 25}
                ],
                "customer": {"name": "Alice", "email": "alice@example.com"}
            }
        });

        let result = query(
            "order.{orderId: id, customerName: customer.name, itemCount: length(items), total: sum(items[*].price)}",
            &data,
            &default_format(),
        )
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["orderId"], "ORD-123");
        assert_eq!(parsed["customerName"], "Alice");
        assert_eq!(parsed["itemCount"], 2);
        assert_eq!(parsed["total"], 35.0);
    }

    #[test]
    fn test_literal_expressions() {
        let data = json!({"x": 5});

        // Backtick literals for comparisons
        let result = query("x > `3`", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        let result = query("x == `5`", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        // String literals
        let result = query("`\"hello\"`", &data, &default_format()).unwrap();
        assert_eq!(result, r#""hello""#);

        // Array literal
        let result = query("`[1, 2, 3]`", &data, &default_format()).unwrap();
        assert_eq!(result, "[1,2,3]");
    }

    #[test]
    fn test_root_array_document() {
        // When the root document is an array, not an object
        let data = json!([
            {"name": "Alice", "score": 85},
            {"name": "Bob", "score": 92},
            {"name": "Carol", "score": 78}
        ]);

        let result = query("[*].name", &data, &default_format()).unwrap();
        assert_eq!(result, r#"["Alice","Bob","Carol"]"#);

        let result = query("[?score > `80`].name", &data, &default_format()).unwrap();
        assert_eq!(result, r#"["Alice","Bob"]"#);

        let result = query("max_by(@, &score).name", &data, &default_format()).unwrap();
        assert_eq!(result, r#""Bob""#);
    }

    #[test]
    fn test_expression_with_at_symbol() {
        // @ represents current node - useful in filters and pipes
        let data = json!({"nums": [1, 2, 3, 4, 5]});

        // Filter using @ for current element
        let result = query("nums[?@ > `2`]", &data, &default_format()).unwrap();
        assert_eq!(result, "[3,4,5]");

        // Pipe with @
        let result = query("nums | [?@ < `4`]", &data, &default_format()).unwrap();
        assert_eq!(result, "[1,2,3]");
    }

    // =========================================================================
    // Expression caching tests
    // =========================================================================

    #[test]
    fn test_cache_repeated_expression() {
        // Same expression should work correctly when cached
        let data1 = json!({"name": "Alice"});
        let data2 = json!({"name": "Bob"});
        let data3 = json!({"name": "Carol"});

        // First call compiles and caches
        let result1 = query("name", &data1, &default_format()).unwrap();
        assert_eq!(result1, r#""Alice""#);

        // Second call uses cache
        let result2 = query("name", &data2, &default_format()).unwrap();
        assert_eq!(result2, r#""Bob""#);

        // Third call still uses cache
        let result3 = query("name", &data3, &default_format()).unwrap();
        assert_eq!(result3, r#""Carol""#);
    }

    #[test]
    fn test_cache_different_expressions() {
        let data = json!({"a": 1, "b": 2, "c": 3});

        // Multiple different expressions should all work
        assert_eq!(query("a", &data, &default_format()).unwrap(), "1");
        assert_eq!(query("b", &data, &default_format()).unwrap(), "2");
        assert_eq!(query("c", &data, &default_format()).unwrap(), "3");

        // And still work on repeated access
        assert_eq!(query("a", &data, &default_format()).unwrap(), "1");
        assert_eq!(query("b", &data, &default_format()).unwrap(), "2");
    }

    #[test]
    fn test_cache_complex_expressions() {
        let data = json!({"users": [
            {"name": "Alice", "score": 85},
            {"name": "Bob", "score": 92}
        ]});

        // Complex expressions should cache correctly
        let expr = "users[?score > `80`] | sort_by(@, &score) | [-1].name";

        let result1 = query(expr, &data, &default_format()).unwrap();
        assert_eq!(result1, r#""Bob""#);

        // Same expression, same result from cache
        let result2 = query(expr, &data, &default_format()).unwrap();
        assert_eq!(result2, r#""Bob""#);
    }

    // =========================================================================
    // Tier 4: Trigonometric functions tests
    // =========================================================================

    #[test]
    fn test_fn_sin() {
        let data = json!({"angle": 0});
        let result = query("sin(angle)", &data, &default_format()).unwrap();
        assert_eq!(result, "0.0");

        // sin(pi/2) = 1
        let data = json!({"angle": 1.5707963267948966});
        let result = query("sin(angle)", &data, &default_format()).unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_fn_cos() {
        let data = json!({"angle": 0});
        let result = query("cos(angle)", &data, &default_format()).unwrap();
        assert_eq!(result, "1.0");

        // cos(pi) = -1
        let data = json!({"angle": 3.141592653589793});
        let result = query("cos(angle)", &data, &default_format()).unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - (-1.0)).abs() < 0.0001);
    }

    #[test]
    fn test_fn_tan() {
        let data = json!({"angle": 0});
        let result = query("tan(angle)", &data, &default_format()).unwrap();
        assert_eq!(result, "0.0");

        // tan(pi/4) = 1
        let data = json!({"angle": 0.7853981633974483});
        let result = query("tan(angle)", &data, &default_format()).unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_fn_asin() {
        // asin(0) = 0
        let data = json!({"val": 0});
        let result = query("asin(val)", &data, &default_format()).unwrap();
        assert_eq!(result, "0.0");

        // asin(1) = pi/2
        let data = json!({"val": 1});
        let result = query("asin(val)", &data, &default_format()).unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - 1.5707963267948966).abs() < 0.0001);

        // asin out of domain returns null
        let data = json!({"val": 2});
        let result = query("asin(val)", &data, &default_format()).unwrap();
        assert_eq!(result, "null");
    }

    #[test]
    fn test_fn_acos() {
        // acos(1) = 0
        let data = json!({"val": 1});
        let result = query("acos(val)", &data, &default_format()).unwrap();
        assert_eq!(result, "0.0");

        // acos(0) = pi/2
        let data = json!({"val": 0});
        let result = query("acos(val)", &data, &default_format()).unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - 1.5707963267948966).abs() < 0.0001);

        // acos out of domain returns null
        let data = json!({"val": 2});
        let result = query("acos(val)", &data, &default_format()).unwrap();
        assert_eq!(result, "null");
    }

    #[test]
    fn test_fn_atan() {
        // atan(0) = 0
        let data = json!({"val": 0});
        let result = query("atan(val)", &data, &default_format()).unwrap();
        assert_eq!(result, "0.0");

        // atan(1) = pi/4
        let data = json!({"val": 1});
        let result = query("atan(val)", &data, &default_format()).unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - 0.7853981633974483).abs() < 0.0001);
    }

    // =========================================================================
    // Tier 4: Sign function tests
    // =========================================================================

    #[test]
    fn test_fn_sign() {
        let data = json!({"pos": 42, "neg": -17, "zero": 0});

        let result = query("sign(pos)", &data, &default_format()).unwrap();
        assert_eq!(result, "1");

        let result = query("sign(neg)", &data, &default_format()).unwrap();
        assert_eq!(result, "-1");

        let result = query("sign(zero)", &data, &default_format()).unwrap();
        assert_eq!(result, "0");

        // Float values
        let data = json!({"f": -3.14});
        let result = query("sign(f)", &data, &default_format()).unwrap();
        assert_eq!(result, "-1");
    }

    // =========================================================================
    // Tier 4: Random/UUID functions tests
    // =========================================================================

    #[test]
    fn test_fn_random() {
        let data = json!({});
        let result = query("random()", &data, &default_format()).unwrap();
        let val: f64 = result.parse().unwrap();
        assert!(val >= 0.0 && val < 1.0);

        // With range
        let result = query("random(`1`, `10`)", &data, &default_format()).unwrap();
        let val: f64 = result.parse().unwrap();
        assert!(val >= 1.0 && val < 10.0);
    }

    #[test]
    fn test_fn_uuid() {
        let data = json!({});
        let result = query("uuid()", &data, &default_format()).unwrap();
        // Remove quotes
        let uuid_str = result.trim_matches('"');
        // UUID v4 format: 8-4-4-4-12 hex chars with dashes
        assert_eq!(uuid_str.len(), 36);
        assert_eq!(uuid_str.chars().filter(|c| *c == '-').count(), 4);
    }

    // =========================================================================
    // Tier 4: Hex encoding tests
    // =========================================================================

    #[test]
    fn test_fn_hex_encode() {
        let data = json!({"s": "hello"});
        let result = query("hex_encode(s)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""68656c6c6f""#);

        let data = json!({"s": ""});
        let result = query("hex_encode(s)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""""#);
    }

    #[test]
    fn test_fn_hex_decode() {
        let data = json!({"s": "68656c6c6f"});
        let result = query("hex_decode(s)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""hello""#);

        // Invalid hex returns null
        let data = json!({"s": "not-hex"});
        let result = query("hex_decode(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "null");
    }

    // =========================================================================
    // Tier 4: String functions tests
    // =========================================================================

    #[test]
    fn test_fn_truncate() {
        let data = json!({"s": "hello world"});

        // Truncate with ellipsis
        let result = query("truncate(s, `5`)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""he...""#);

        // Custom suffix
        let result = query(r#"truncate(s, `8`, `"--"`)"#, &data, &default_format()).unwrap();
        assert_eq!(result, r#""hello --""#);

        // No truncation needed
        let result = query("truncate(s, `50`)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""hello world""#);
    }

    #[test]
    fn test_fn_trim_left() {
        let data = json!({"s": "   hello"});
        let result = query("trim_left(s)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""hello""#);

        let data = json!({"s": "hello"});
        let result = query("trim_left(s)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""hello""#);
    }

    #[test]
    fn test_fn_trim_right() {
        let data = json!({"s": "hello   "});
        let result = query("trim_right(s)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""hello""#);

        let data = json!({"s": "hello"});
        let result = query("trim_right(s)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""hello""#);
    }

    // =========================================================================
    // Tier 4: Validation functions tests
    // =========================================================================

    #[test]
    fn test_fn_is_empty() {
        // Empty string
        let data = json!({"s": ""});
        let result = query("is_empty(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        // Non-empty string
        let data = json!({"s": "hello"});
        let result = query("is_empty(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "false");

        // Empty array
        let data = json!({"arr": []});
        let result = query("is_empty(arr)", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        // Non-empty array
        let data = json!({"arr": [1, 2]});
        let result = query("is_empty(arr)", &data, &default_format()).unwrap();
        assert_eq!(result, "false");

        // Empty object
        let data = json!({"obj": {}});
        let result = query("is_empty(obj)", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        // Non-empty object
        let data = json!({"obj": {"a": 1}});
        let result = query("is_empty(obj)", &data, &default_format()).unwrap();
        assert_eq!(result, "false");

        // Null is empty
        let data = json!({"n": null});
        let result = query("is_empty(n)", &data, &default_format()).unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_fn_is_blank() {
        // Empty string is blank
        let data = json!({"s": ""});
        let result = query("is_blank(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        // Whitespace only is blank
        let data = json!({"s": "   "});
        let result = query("is_blank(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        // Tabs/newlines are blank
        let data = json!({"s": " \t\n "});
        let result = query("is_blank(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        // Non-blank string
        let data = json!({"s": "  hello  "});
        let result = query("is_blank(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "false");

        // Non-string returns null
        let data = json!({"n": 42});
        let result = query("is_blank(n)", &data, &default_format()).unwrap();
        assert_eq!(result, "null");
    }

    #[test]
    fn test_fn_is_json() {
        // Valid JSON
        let data = json!({"s": r#"{"a": 1}"#});
        let result = query("is_json(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        // Valid JSON array
        let data = json!({"s": "[1, 2, 3]"});
        let result = query("is_json(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "true");

        // Invalid JSON
        let data = json!({"s": "{not json}"});
        let result = query("is_json(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "false");

        // Non-string returns null
        let data = json!({"n": 42});
        let result = query("is_json(n)", &data, &default_format()).unwrap();
        assert_eq!(result, "null");
    }

    // =========================================================================
    // Tier 5: Additional math functions
    // =========================================================================

    #[test]
    fn test_fn_atan2() {
        let data = json!({"y": 1, "x": 1});
        let result = query("atan2(y, x)", &data, &default_format()).unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - 0.7853981633974483).abs() < 0.0001); // pi/4

        let data = json!({"y": 0, "x": 1});
        let result = query("atan2(y, x)", &data, &default_format()).unwrap();
        assert_eq!(result, "0.0");
    }

    #[test]
    fn test_fn_deg_to_rad() {
        let data = json!({"deg": 180});
        let result = query("deg_to_rad(deg)", &data, &default_format()).unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - std::f64::consts::PI).abs() < 0.0001);

        let data = json!({"deg": 90});
        let result = query("deg_to_rad(deg)", &data, &default_format()).unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - std::f64::consts::FRAC_PI_2).abs() < 0.0001);
    }

    #[test]
    fn test_fn_rad_to_deg() {
        let data = json!({"rad": 3.141592653589793});
        let result = query("rad_to_deg(rad)", &data, &default_format()).unwrap();
        let val: f64 = result.parse().unwrap();
        assert!((val - 180.0).abs() < 0.0001);
    }

    // =========================================================================
    // Tier 5: Array functions
    // =========================================================================

    #[test]
    fn test_fn_nth() {
        let data = json!({"arr": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]});
        // Every 2nd element (indices 0, 2, 4, 6, 8)
        let result = query("nth(arr, `2`)", &data, &default_format()).unwrap();
        assert_eq!(result, "[1,3,5,7,9]");

        // Every 3rd element
        let result = query("nth(arr, `3`)", &data, &default_format()).unwrap();
        assert_eq!(result, "[1,4,7,10]");
    }

    #[test]
    fn test_fn_interleave() {
        let data = json!({"a": [1, 2, 3], "b": ["a", "b", "c"]});
        let result = query("interleave(a, b)", &data, &default_format()).unwrap();
        assert_eq!(result, r#"[1,"a",2,"b",3,"c"]"#);

        // Unequal lengths
        let data = json!({"a": [1, 2], "b": ["a", "b", "c", "d"]});
        let result = query("interleave(a, b)", &data, &default_format()).unwrap();
        assert_eq!(result, r#"[1,"a",2,"b","c","d"]"#);
    }

    #[test]
    fn test_fn_rotate() {
        let data = json!({"arr": [1, 2, 3, 4, 5]});
        // Rotate left by 2
        let result = query("rotate(arr, `2`)", &data, &default_format()).unwrap();
        assert_eq!(result, "[3,4,5,1,2]");

        // Rotate right (negative)
        let result = query("rotate(arr, `-1`)", &data, &default_format()).unwrap();
        assert_eq!(result, "[5,1,2,3,4]");
    }

    #[test]
    fn test_fn_partition() {
        let data = json!({"arr": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]});
        // Split into 3 parts
        let result = query("partition(arr, `3`)", &data, &default_format()).unwrap();
        assert_eq!(result, "[[1,2,3,4],[5,6,7],[8,9,10]]");

        // Split into 2 parts
        let result = query("partition(arr, `2`)", &data, &default_format()).unwrap();
        assert_eq!(result, "[[1,2,3,4,5],[6,7,8,9,10]]");
    }

    // =========================================================================
    // Tier 5: Object functions
    // =========================================================================

    #[test]
    fn test_fn_invert() {
        let data = json!({"a": 1, "b": 2, "c": 3});
        let result = query("invert(@)", &data, &default_format()).unwrap();
        assert_eq!(result, r#"{"1":"a","2":"b","3":"c"}"#);
    }

    #[test]
    fn test_fn_rename_keys() {
        let data = json!({"obj": {"old_name": 1, "keep": 2}, "map": {"old_name": "new_name"}});
        let result = query("rename_keys(obj, map)", &data, &default_format()).unwrap();
        assert_eq!(result, r#"{"keep":2,"new_name":1}"#);
    }

    #[test]
    fn test_fn_flatten_keys() {
        let data = json!({"a": {"b": {"c": 1}}, "d": 2});
        let result = query("flatten_keys(@)", &data, &default_format()).unwrap();
        assert_eq!(result, r#"{"a.b.c":1,"d":2}"#);
    }

    #[test]
    fn test_fn_unflatten_keys() {
        let data = json!({"a.b.c": 1, "a.b.d": 2, "e": 3});
        let result = query("unflatten_keys(@)", &data, &default_format()).unwrap();
        assert_eq!(result, r#"{"a":{"b":{"c":1,"d":2}},"e":3}"#);
    }

    // =========================================================================
    // Tier 5: Encoding functions
    // =========================================================================

    #[test]
    fn test_fn_json_encode() {
        let data = json!({"obj": {"a": 1, "b": "hello"}});
        let result = query("json_encode(obj)", &data, &default_format()).unwrap();
        // Result is a JSON string containing the encoded object
        // The output will be escaped since it's a string containing JSON
        assert!(result.contains("a") && result.contains("1"));
        assert!(result.contains("b") && result.contains("hello"));
    }

    #[test]
    fn test_fn_json_decode() {
        let data = json!({"s": "{\"a\":1,\"b\":2}"});
        let result = query("json_decode(s)", &data, &default_format()).unwrap();
        assert_eq!(result, r#"{"a":1,"b":2}"#);

        // Invalid JSON returns null
        let data = json!({"s": "not json"});
        let result = query("json_decode(s)", &data, &default_format()).unwrap();
        assert_eq!(result, "null");
    }

    // =========================================================================
    // Tier 5: Utility functions
    // =========================================================================

    #[test]
    fn test_fn_path_join() {
        let data = json!({"parts": ["home", "user", "file.txt"]});
        let result = query("path_join(parts)", &data, &default_format()).unwrap();
        assert!(result.contains("home") && result.contains("user") && result.contains("file.txt"));
    }

    #[test]
    fn test_fn_coalesce() {
        let data = json!({"a": null, "b": null, "c": "found", "d": "also"});
        let result = query("coalesce(a, b, c, d)", &data, &default_format()).unwrap();
        assert_eq!(result, r#""found""#);

        // All null
        let data = json!({"a": null, "b": null});
        let result = query("coalesce(a, b)", &data, &default_format()).unwrap();
        assert_eq!(result, "null");

        // First is non-null
        let data = json!({"a": 1, "b": 2});
        let result = query("coalesce(a, b)", &data, &default_format()).unwrap();
        assert_eq!(result, "1");
    }

    // =========================================================================
    // Tier 6: Regex functions
    // =========================================================================

    #[test]
    fn test_fn_regex_match() {
        let data = json!({"email": "user@example.com"});
        let result = query(
            r#"regex_match(email, `"^[^@]+@[^@]+\\.[^@]+$"`)"#,
            &data,
            &default_format(),
        )
        .unwrap();
        assert_eq!(result, "true");

        let data = json!({"email": "not-an-email"});
        let result = query(
            r#"regex_match(email, `"^[^@]+@[^@]+\\.[^@]+$"`)"#,
            &data,
            &default_format(),
        )
        .unwrap();
        assert_eq!(result, "false");
    }

    #[test]
    fn test_fn_regex_extract() {
        let data = json!({"url": "https://example.com/path"});
        let result = query(
            r#"regex_extract(url, `"https?://([^/]+)(.*)"`)"#,
            &data,
            &default_format(),
        )
        .unwrap();
        assert!(result.contains("example.com"));
        assert!(result.contains("/path"));

        // No match returns null
        let data = json!({"s": "hello"});
        let result = query(r#"regex_extract(s, `"\\d+"`)"#, &data, &default_format()).unwrap();
        assert_eq!(result, "null");
    }

    #[test]
    fn test_fn_regex_replace() {
        let data = json!({"s": "hello 123 world 456"});
        let result = query(
            r#"regex_replace(s, `"\\d+"`, `"NUM"`)"#,
            &data,
            &default_format(),
        )
        .unwrap();
        assert_eq!(result, r#""hello NUM world NUM""#);

        // Replace with capture group reference
        let data = json!({"s": "hello world"});
        let result = query(
            r#"regex_replace(s, `"(\\w+)"`, `"[$1]"`)"#,
            &data,
            &default_format(),
        )
        .unwrap();
        assert_eq!(result, r#""[hello] [world]""#);
    }

    // =========================================================================
    // Tier 6: String functions
    // =========================================================================

    #[test]
    fn test_fn_wrap() {
        let data = json!({"s": "the quick brown fox jumps over the lazy dog"});
        let result = query("wrap(s, `10`)", &data, &default_format()).unwrap();
        // Should have newlines
        assert!(result.contains("\\n"));
    }

    #[test]
    fn test_fn_format() {
        let data = json!({"name": "Alice", "age": 30});
        let result = query(
            r#"format(`"{0} is {1} years old"`, name, age)"#,
            &data,
            &default_format(),
        )
        .unwrap();
        assert_eq!(result, r#""Alice is 30 years old""#);

        // Multiple same placeholders
        let data = json!({"x": "hello"});
        let result = query(r#"format(`"{0} {0}"`, x)"#, &data, &default_format()).unwrap();
        assert_eq!(result, r#""hello hello""#);
    }

    // =========================================================================
    // Tier 6: Array functions
    // =========================================================================

    #[test]
    fn test_fn_shuffle_with_seed() {
        let data = json!({"arr": [1, 2, 3, 4, 5]});
        // With seed, should be deterministic
        let result1 = query("shuffle(arr, `42`)", &data, &default_format()).unwrap();
        let result2 = query("shuffle(arr, `42`)", &data, &default_format()).unwrap();
        assert_eq!(result1, result2);

        // Different seed, different result
        let result3 = query("shuffle(arr, `999`)", &data, &default_format()).unwrap();
        assert_ne!(result1, result3);
    }

    #[test]
    fn test_fn_sample_with_seed() {
        let data = json!({"arr": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]});
        // Sample 3 elements with seed
        let result = query("sample(arr, `3`, `42`)", &data, &default_format()).unwrap();
        // Should be an array with 3 elements
        let parsed: Vec<i32> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 3);

        // Deterministic with same seed
        let result2 = query("sample(arr, `3`, `42`)", &data, &default_format()).unwrap();
        assert_eq!(result, result2);
    }

    #[test]
    fn test_fn_cartesian() {
        let data = json!({"a": [1, 2], "b": ["x", "y"]});
        let result = query("cartesian(a, b)", &data, &default_format()).unwrap();
        assert_eq!(result, r#"[[1,"x"],[1,"y"],[2,"x"],[2,"y"]]"#);

        // Empty array
        let data = json!({"a": [], "b": [1, 2]});
        let result = query("cartesian(a, b)", &data, &default_format()).unwrap();
        assert_eq!(result, "[]");
    }

    // =========================================================================
    // URL parsing
    // =========================================================================

    #[test]
    fn test_fn_url_parse() {
        // Full URL with all components
        let data = json!({"url": "https://user:pass@example.com:8080/path/to/resource?foo=bar&baz=qux#section1"});
        let result = query("url_parse(url)", &data, &default_format()).unwrap();
        assert!(result.contains("\"scheme\":\"https\""));
        assert!(result.contains("\"host\":\"example.com\""));
        assert!(result.contains("\"port\":8080"));
        assert!(result.contains("\"path\":\"/path/to/resource\""));
        assert!(result.contains("\"query\":\"foo=bar&baz=qux\""));
        assert!(result.contains("\"fragment\":\"section1\""));
        assert!(result.contains("\"username\":\"user\""));
        assert!(result.contains("\"password\":\"pass\""));

        // Access specific component
        let result = query("url_parse(url).host", &data, &default_format()).unwrap();
        assert_eq!(result, "\"example.com\"");

        let result = query("url_parse(url).port", &data, &default_format()).unwrap();
        assert_eq!(result, "8080");

        // Simple URL without optional components
        let data = json!({"url": "https://example.com/path"});
        let result = query("url_parse(url)", &data, &default_format()).unwrap();
        assert!(result.contains("\"scheme\":\"https\""));
        assert!(result.contains("\"host\":\"example.com\""));
        assert!(result.contains("\"port\":null"));
        assert!(result.contains("\"query\":null"));
        assert!(result.contains("\"fragment\":null"));

        // Invalid URL returns null
        let data = json!({"url": "not a valid url"});
        let result = query("url_parse(url)", &data, &default_format()).unwrap();
        assert_eq!(result, "null");
    }

    #[test]
    fn test_fn_url_parse_origin() {
        let data = json!({"url": "https://example.com:8080/path"});
        let result = query("url_parse(url).origin", &data, &default_format()).unwrap();
        assert_eq!(result, "\"https://example.com:8080\"");

        // Without explicit port
        let data = json!({"url": "https://example.com/path"});
        let result = query("url_parse(url).origin", &data, &default_format()).unwrap();
        assert_eq!(result, "\"https://example.com\"");
    }

    // =========================================================================
    // Time functions with fallback
    // =========================================================================

    #[test]
    fn test_fn_now_fallback() {
        let data = json!({});
        // With fallback - deterministic
        let result = query("now(`1700000000`)", &data, &default_format()).unwrap();
        let ts: f64 = result.parse().unwrap();
        assert_eq!(ts, 1700000000.0);

        // Without fallback - returns current time (non-deterministic, just check it's a number)
        let result = query("now()", &data, &default_format()).unwrap();
        let ts: f64 = result.parse().unwrap();
        assert!(ts > 1700000000.0); // Should be after Nov 2023
    }

    #[test]
    fn test_fn_now_ms_fallback() {
        let data = json!({});
        // With fallback - deterministic
        let result = query("now_ms(`1700000000000`)", &data, &default_format()).unwrap();
        let ts: f64 = result.parse().unwrap();
        assert_eq!(ts, 1700000000000.0);

        // Without fallback - returns current time in ms
        let result = query("now_ms()", &data, &default_format()).unwrap();
        let ts: f64 = result.parse().unwrap();
        assert!(ts > 1700000000000.0); // Should be after Nov 2023
    }

    // =========================================================================
    // RESP3 conversion tests
    // =========================================================================

    #[test]
    fn test_resp3_null() {
        let data = json!({"a": null});
        let result = query_to_resp3("a", &data).unwrap();
        assert!(matches!(result, RedisValue::Null));
    }

    #[test]
    fn test_resp3_bool() {
        let data = json!({"t": true, "f": false});

        let result = query_to_resp3("t", &data).unwrap();
        assert!(matches!(result, RedisValue::Bool(true)));

        let result = query_to_resp3("f", &data).unwrap();
        assert!(matches!(result, RedisValue::Bool(false)));
    }

    #[test]
    fn test_resp3_integer() {
        let data = json!({"n": 42});
        let result = query_to_resp3("n", &data).unwrap();
        assert!(matches!(result, RedisValue::Integer(42)));
    }

    #[test]
    fn test_resp3_float() {
        let data = json!({"n": 3.14});
        let result = query_to_resp3("n", &data).unwrap();
        match result {
            RedisValue::Float(f) => assert!((f - 3.14).abs() < 0.001),
            _ => panic!("Expected Float"),
        }
    }

    #[test]
    fn test_resp3_string() {
        let data = json!({"s": "hello"});
        let result = query_to_resp3("s", &data).unwrap();
        assert!(matches!(result, RedisValue::BulkString(s) if s == "hello"));
    }

    #[test]
    fn test_resp3_array() {
        let data = json!({"arr": [1, 2, 3]});
        let result = query_to_resp3("arr", &data).unwrap();
        match result {
            RedisValue::Array(arr) => {
                assert_eq!(arr.len(), 3);
                assert!(matches!(arr[0], RedisValue::Integer(1)));
                assert!(matches!(arr[1], RedisValue::Integer(2)));
                assert!(matches!(arr[2], RedisValue::Integer(3)));
            }
            _ => panic!("Expected Array"),
        }
    }

    #[test]
    fn test_resp3_object() {
        let data = json!({"obj": {"a": 1, "b": "two"}});
        let result = query_to_resp3("obj", &data).unwrap();
        match result {
            RedisValue::Map(map) => {
                assert_eq!(map.len(), 2);
                // Map keys are RedisValueKey::String
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_resp3_nested() {
        let data = json!({
            "users": [
                {"name": "Alice", "active": true},
                {"name": "Bob", "active": false}
            ]
        });

        // Get names as array
        let result = query_to_resp3("users[*].name", &data).unwrap();
        match result {
            RedisValue::Array(arr) => {
                assert_eq!(arr.len(), 2);
                assert!(matches!(&arr[0], RedisValue::BulkString(s) if s == "Alice"));
                assert!(matches!(&arr[1], RedisValue::BulkString(s) if s == "Bob"));
            }
            _ => panic!("Expected Array"),
        }

        // Filter and get single value
        let result = query_to_resp3("users[?active].name | [0]", &data).unwrap();
        assert!(matches!(result, RedisValue::BulkString(s) if s == "Alice"));
    }
}
