/*
 * Copyright (c) 2006-Present, Redis Ltd.
 * All rights reserved.
 *
 * Licensed under your choice of (a) the Redis Source Available License 2.0
 * (RSALv2); or (b) the Server Side Public License v1 (SSPLv1); or (c) the
 * GNU Affero General Public License v3 (AGPLv3).
 */

//! Custom JMESPath functions for RedisJSON
//!
//! This module provides 150+ extensions to JMESPath beyond the standard 26 built-in
//! functions. These functions are registered with a custom Runtime and are available
//! in all JSON.JMESPATH queries.
//!
//! Functions are provided by the `jmespath_extensions` crate and can be selectively
//! enabled/disabled via module load arguments:
//!
//! ```text
//! MODULE LOAD redisjson.so jmespath-allow "string,array,math" jmespath-deny "random,datetime"
//! ```
//!
//! **Note:** Configuration is read at module load time. Changes require a module restart.
//!
//! ## Function Categories
//!
//! - **string**: lower, upper, trim, split, replace, pad_left, pad_right, etc.
//! - **array**: unique, zip, chunk, take, drop, flatten_deep, compact, range, etc.
//! - **object**: items, from_items, pick, omit, deep_merge
//! - **math**: round, floor_fn, ceil_fn, abs_fn, mod_fn, pow, sqrt, log, etc.
//! - **type**: to_string, to_number, to_boolean, type_of, is_string, etc.
//! - **utility**: now, now_millis, default, coalesce, format, if, etc.
//! - **hash**: md5, sha1, sha256, sha512, crc32
//! - **encoding**: base64_encode, base64_decode, hex_encode, hex_decode
//! - **regex**: regex_match, regex_replace, regex_extract
//! - **url**: url_parse, url_encode, url_decode
//! - **random**: random, shuffle, sample, uuid
//! - **validation**: is_email, is_url, is_ipv4, is_ipv6, is_uuid
//! - **path**: path_basename, path_dirname, path_ext, path_join
//! - **datetime**: now, now_millis, parse_date, format_date, date_add, date_diff
//! - **fuzzy**: levenshtein, jaro, jaro_winkler, sorensen_dice
//! - **phonetic**: soundex, metaphone, double_metaphone, nysiis
//! - **expression**: map_expr, filter_expr, find_expr, any_expr, all_expr, etc.
//! - **geo**: geo_distance, geo_bearing
//! - **semver**: semver_parse, semver_compare, semver_satisfies
//! - **network**: ip_to_int, int_to_ip, cidr_contains, is_private_ip
//! - **ids**: nanoid, ulid
//! - **text**: word_count, char_count, sentence_count
//! - **duration**: parse_duration, format_duration
//! - **color**: hex_to_rgb, rgb_to_hex
//! - **computing**: bytes_to_human, human_to_bytes
//!
//! For detailed documentation, see the `jmespath_extensions` crate.

use jmespath::Runtime;
use std::collections::HashSet;
use std::sync::{LazyLock, RwLock};

/// All available function categories (for introspection/debugging)
#[allow(dead_code)]
pub const ALL_CATEGORIES: &[&str] = &[
    "string",
    "array",
    "object",
    "math",
    "type",
    "utility",
    "path",
    "validation",
    "hash",
    "encoding",
    "url",
    "regex",
    "random",
    "datetime",
    "fuzzy",
    "expression",
    "phonetic",
    "geo",
    "semver",
    "network",
    "ids",
    "text",
    "duration",
    "color",
    "computing",
];

/// Configuration for JMESPath function registration
#[derive(Debug, Clone)]
pub struct JmespathConfig {
    /// Categories to allow (empty = all allowed)
    pub allow: HashSet<String>,
    /// Categories to deny (takes precedence over allow)
    pub deny: HashSet<String>,
}

impl Default for JmespathConfig {
    fn default() -> Self {
        Self {
            allow: HashSet::new(), // empty = allow all
            deny: HashSet::new(),
        }
    }
}

impl JmespathConfig {
    /// Check if a category is enabled
    pub fn is_category_enabled(&self, category: &str) -> bool {
        // Deny takes precedence
        if self.deny.contains(category) || self.deny.contains("*") {
            return false;
        }
        // If allow is empty or contains "*", allow all (except denied)
        if self.allow.is_empty() || self.allow.contains("*") {
            return true;
        }
        // Otherwise, must be explicitly allowed
        self.allow.contains(category)
    }

    /// Parse a comma-separated category string
    pub fn parse_categories(s: &str) -> HashSet<String> {
        s.split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// Global JMESPath configuration
pub static JMESPATH_CONFIG: LazyLock<RwLock<JmespathConfig>> =
    LazyLock::new(|| RwLock::new(JmespathConfig::default()));

/// Global JMESPath runtime with Redis-specific functions.
///
/// This runtime includes all 26 standard JMESPath functions plus
/// 150+ custom extensions from the jmespath_extensions crate.
///
/// **Note:** Configuration is read at module load time. Changes to
/// `json.jmespath-allow` or `json.jmespath-deny` require a module restart.
pub static REDIS_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    let config = JMESPATH_CONFIG.read().unwrap();
    build_runtime(&config)
});

/// Update the allow list (config only - runtime not rebuilt until restart)
pub fn set_allow_categories(categories: &str) {
    let mut config = JMESPATH_CONFIG.write().unwrap();
    config.allow = JmespathConfig::parse_categories(categories);
}

/// Update the deny list (config only - runtime not rebuilt until restart)
pub fn set_deny_categories(categories: &str) {
    let mut config = JMESPATH_CONFIG.write().unwrap();
    config.deny = JmespathConfig::parse_categories(categories);
}

/// Get current allow categories as comma-separated string (for introspection/debugging)
#[allow(dead_code)]
pub fn get_allow_categories() -> String {
    let config = JMESPATH_CONFIG.read().unwrap();
    if config.allow.is_empty() {
        "*".to_string()
    } else {
        let mut cats: Vec<_> = config.allow.iter().cloned().collect();
        cats.sort();
        cats.join(",")
    }
}

/// Get current deny categories as comma-separated string (for introspection/debugging)
#[allow(dead_code)]
pub fn get_deny_categories() -> String {
    let config = JMESPATH_CONFIG.read().unwrap();
    if config.deny.is_empty() {
        "".to_string()
    } else {
        let mut cats: Vec<_> = config.deny.iter().cloned().collect();
        cats.sort();
        cats.join(",")
    }
}

/// Build a JMESPath runtime with the given configuration
pub fn build_runtime(config: &JmespathConfig) -> Runtime {
    use jmespath_extensions::*;

    let mut runtime = Runtime::new();
    runtime.register_builtin_functions();

    // Register each category if enabled
    if config.is_category_enabled("string") {
        string::register(&mut runtime);
    }
    if config.is_category_enabled("array") {
        array::register(&mut runtime);
    }
    if config.is_category_enabled("object") {
        object::register(&mut runtime);
    }
    if config.is_category_enabled("math") {
        math::register(&mut runtime);
    }
    if config.is_category_enabled("type") {
        type_conv::register(&mut runtime);
    }
    if config.is_category_enabled("utility") {
        utility::register(&mut runtime);
    }
    if config.is_category_enabled("path") {
        path::register(&mut runtime);
    }
    if config.is_category_enabled("validation") {
        validation::register(&mut runtime);
    }
    if config.is_category_enabled("hash") {
        hash::register(&mut runtime);
    }
    if config.is_category_enabled("encoding") {
        encoding::register(&mut runtime);
    }
    if config.is_category_enabled("url") {
        url_fns::register(&mut runtime);
    }
    if config.is_category_enabled("regex") {
        regex_fns::register(&mut runtime);
    }
    if config.is_category_enabled("random") {
        random::register(&mut runtime);
    }
    if config.is_category_enabled("datetime") {
        datetime::register(&mut runtime);
    }
    if config.is_category_enabled("fuzzy") {
        fuzzy::register(&mut runtime);
    }
    if config.is_category_enabled("expression") {
        expression::register(&mut runtime);
    }
    if config.is_category_enabled("phonetic") {
        phonetic::register(&mut runtime);
    }
    if config.is_category_enabled("geo") {
        geo::register(&mut runtime);
    }
    if config.is_category_enabled("semver") {
        semver_fns::register(&mut runtime);
    }
    if config.is_category_enabled("network") {
        network::register(&mut runtime);
    }
    if config.is_category_enabled("ids") {
        ids::register(&mut runtime);
    }
    if config.is_category_enabled("text") {
        text::register(&mut runtime);
    }
    if config.is_category_enabled("duration") {
        duration::register(&mut runtime);
    }
    if config.is_category_enabled("color") {
        color::register(&mut runtime);
    }
    if config.is_category_enabled("computing") {
        computing::register(&mut runtime);
    }

    runtime
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_allows_all() {
        let config = JmespathConfig::default();
        assert!(config.is_category_enabled("string"));
        assert!(config.is_category_enabled("random"));
        assert!(config.is_category_enabled("anything"));
    }

    #[test]
    fn test_config_deny_takes_precedence() {
        let mut config = JmespathConfig::default();
        config.allow.insert("*".to_string());
        config.deny.insert("random".to_string());

        assert!(config.is_category_enabled("string"));
        assert!(!config.is_category_enabled("random"));
    }

    #[test]
    fn test_config_explicit_allow() {
        let mut config = JmespathConfig::default();
        config.allow.insert("string".to_string());
        config.allow.insert("array".to_string());

        assert!(config.is_category_enabled("string"));
        assert!(config.is_category_enabled("array"));
        assert!(!config.is_category_enabled("random"));
    }

    #[test]
    fn test_config_deny_all() {
        let mut config = JmespathConfig::default();
        config.deny.insert("*".to_string());

        assert!(!config.is_category_enabled("string"));
        assert!(!config.is_category_enabled("random"));
    }

    #[test]
    fn test_parse_categories() {
        let cats = JmespathConfig::parse_categories("string, array, MATH");
        assert!(cats.contains("string"));
        assert!(cats.contains("array"));
        assert!(cats.contains("math"));
        assert_eq!(cats.len(), 3);
    }

    #[test]
    fn test_build_runtime_with_restrictions() {
        let mut config = JmespathConfig::default();
        config.allow.insert("string".to_string());

        let runtime = build_runtime(&config);

        // String function should work
        let expr = runtime.compile("lower(`\"HELLO\"`)").unwrap();
        let result = expr.search(&serde_json::json!({})).unwrap();
        assert_eq!(result.as_string().unwrap(), "hello");
    }

    #[test]
    fn test_get_set_categories() {
        // These modify global state, so just test the format
        let allow = get_allow_categories();
        assert!(allow == "*" || allow.contains(",") || ALL_CATEGORIES.contains(&allow.as_str()));
    }
}
