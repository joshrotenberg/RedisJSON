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
//! This module provides 129 Redis-specific extensions to JMESPath beyond the
//! standard 26 built-in functions. These functions are registered with a
//! custom Runtime and are available in all JSON.JMESPATH queries.
//!
//! The functions are provided by the `jmespath_extensions` crate and include:
//!
//! - **String Functions (23)**: lower, upper, trim, trim_left, trim_right, split, replace,
//!   pad_left, pad_right, substr, slice, find_first, find_last, concat, capitalize,
//!   title, repeat, upper_case, lower_case, title_case, camel_case, snake_case,
//!   kebab_case, url_encode, url_decode
//!
//! - **Array Functions (19)**: unique, zip, chunk, take, drop, flatten_deep, compact,
//!   range, index_at, includes, find_index, first, last, difference, intersection,
//!   union, group_by, frequencies, mode
//!
//! - **Object Functions (5)**: items, from_items, pick, omit, deep_merge
//!
//! - **Math/Statistics Functions (14)**: round, floor_fn, ceil_fn, abs_fn, mod_fn,
//!   pow, sqrt, log, clamp, median, percentile, variance, stddev, sum_of
//!
//! - **Type Functions (10)**: to_string, to_number, to_boolean, to_array, to_object,
//!   type_of, is_string, is_number, is_boolean, is_array, is_object, is_null
//!
//! - **Utility Functions (10)**: now, now_ms, default, coalesce, format, if_fn,
//!   unless, empty, size_of, debug
//!
//! - **Hash Functions (5)**: md5, sha1, sha256, sha512, crc32
//!
//! - **Encoding Functions (4)**: base64_encode, base64_decode, hex_encode, hex_decode
//!
//! - **Regex Functions (4)**: regex_match, regex_replace, regex_extract, regex_split
//!
//! - **URL Functions (2)**: url_parse, (url_encode/url_decode in string section)
//!
//! - **UUID Functions (2)**: uuid, is_uuid
//!
//! - **Random Functions (3)**: random, random_int, shuffle
//!
//! - **Validation Functions (4)**: is_email, is_url, is_ipv4, is_ipv6
//!
//! - **Path Functions (3)**: path_basename, path_dirname, path_ext
//!
//! For detailed documentation, see the `jmespath_extensions` crate or the
//! RedisJSON JMESPath documentation.

use jmespath::Runtime;
use std::sync::LazyLock;

/// Custom JMESPath runtime with Redis-specific functions.
///
/// This runtime includes all 26 standard JMESPath functions plus
/// 129 custom Redis extensions from the jmespath_extensions crate.
pub static REDIS_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    let mut runtime = Runtime::new();
    runtime.register_builtin_functions();
    jmespath_extensions::register_all(&mut runtime);
    runtime
});
