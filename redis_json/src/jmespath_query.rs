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
