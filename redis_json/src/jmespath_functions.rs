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
//! This module provides 61 Redis-specific extensions to JMESPath beyond the
//! standard 26 built-in functions. These functions are registered with a
//! custom Runtime and are available in all JSON.JMESPATH queries.
//!
//! ## String Functions (15)
//!
//! | Function | Description |
//! |----------|-------------|
//! | `lower(string)` | Convert to lowercase |
//! | `upper(string)` | Convert to uppercase |
//! | `trim(string)` | Remove leading/trailing whitespace |
//! | `capitalize(string)` | Capitalize first letter |
//! | `title(string)` | Capitalize each word |
//! | `split(string, delim)` | Split string into array |
//! | `replace(string, old, new)` | Replace all occurrences |
//! | `repeat(string, count)` | Repeat string n times |
//! | `pad_left(string, width, char)` | Left-pad string |
//! | `pad_right(string, width, char)` | Right-pad string |
//! | `substr(string, start, len?)` | Extract substring |
//! | `slice(string, start, end?)` | Extract by indices |
//! | `index_of(string, search)` | Find first occurrence |
//! | `last_index_of(string, search)` | Find last occurrence |
//! | `concat(array, sep?)` | Join strings |
//!
//! ## Array Functions (17)
//!
//! | Function | Description |
//! |----------|-------------|
//! | `unique(array)` | Remove duplicates |
//! | `zip(arr1, arr2)` | Pair elements |
//! | `chunk(array, size)` | Split into chunks |
//! | `take(array, n)` | First n elements |
//! | `drop(array, n)` | Skip n elements |
//! | `flatten_deep(array)` | Recursive flatten |
//! | `compact(array)` | Remove null/false |
//! | `range(start, end, step?)` | Generate sequence |
//! | `index_at(array, idx)` | Get by index (neg ok) |
//! | `includes(array, val)` | Check membership |
//! | `find_index(array, val)` | Find element index |
//! | `first(array)` | First element or null |
//! | `last(array)` | Last element or null |
//! | `difference(arr1, arr2)` | Set difference |
//! | `intersection(arr1, arr2)` | Set intersection |
//! | `union(arr1, arr2)` | Set union |
//! | `group_by(array, field)` | Group by field value |
//!
//! ## Object Functions (4)
//!
//! | Function | Description |
//! |----------|-------------|
//! | `entries(object)` | Convert to [{key, value}] |
//! | `from_entries(array)` | Convert [{key, value}] to object |
//! | `pick(object, keys)` | Select specific keys |
//! | `omit(object, keys)` | Exclude specific keys |
//!
//! ## Math/Statistics Functions (11)
//!
//! | Function | Description |
//! |----------|-------------|
//! | `round(n, precision?)` | Round to decimals |
//! | `floor_fn(n)` | Round down |
//! | `ceil_fn(n)` | Round up |
//! | `abs_fn(n)` | Absolute value |
//! | `mod_fn(n, divisor)` | Modulo |
//! | `pow(base, exp)` | Exponentiation |
//! | `sqrt(n)` | Square root |
//! | `log(n, base?)` | Logarithm |
//! | `clamp(n, min, max)` | Constrain to range |
//! | `median(array)` | Median value |
//! | `percentile(array, p)` | Nth percentile (0-100) |
//!
//! ## Type Functions (10)
//!
//! | Function | Description |
//! |----------|-------------|
//! | `to_string(any)` | Convert to string |
//! | `to_number(any)` | Convert to number |
//! | `to_boolean(any)` | Convert to boolean |
//! | `type_of(any)` | Get type name |
//! | `is_string(any)` | Check if string |
//! | `is_number(any)` | Check if number |
//! | `is_boolean(any)` | Check if boolean |
//! | `is_array(any)` | Check if array |
//! | `is_object(any)` | Check if object |
//! | `is_null(any)` | Check if null |
//!
//! ## Utility/Conditional Functions (4)
//!
//! | Function | Description |
//! |----------|-------------|
//! | `now()` | Unix timestamp (seconds) |
//! | `now_ms()` | Unix timestamp (milliseconds) |
//! | `default(value, fallback)` | Return fallback if null |
//! | `if(cond, then, else)` | Ternary conditional |
//!
//! ## Portability Note
//!
//! Queries using these custom functions will NOT work in other JMESPath
//! implementations (Python, JavaScript, Go, etc.). For portable queries,
//! use only the standard 26 JMESPath functions.

use std::collections::HashSet;
use std::rc::Rc;
use std::sync::LazyLock;

use jmespath::functions::{ArgumentType, Function, Signature};
use jmespath::{Context, ErrorReason, JmespathError, Rcvar, Runtime, Variable};

/// Custom JMESPath runtime with Redis-specific functions.
///
/// This runtime includes all 26 standard JMESPath functions plus
/// custom Redis extensions.
pub static REDIS_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    let mut runtime = Runtime::new();
    runtime.register_builtin_functions();

    // Register custom functions - String operations
    runtime.register_function("lower", Box::new(LowerFn::new()));
    runtime.register_function("upper", Box::new(UpperFn::new()));
    runtime.register_function("trim", Box::new(TrimFn::new()));
    runtime.register_function("split", Box::new(SplitFn::new()));
    runtime.register_function("replace", Box::new(ReplaceFn::new()));
    runtime.register_function("pad_left", Box::new(PadLeftFn::new()));
    runtime.register_function("pad_right", Box::new(PadRightFn::new()));
    runtime.register_function("substr", Box::new(SubstrFn::new()));

    // Register custom functions - Array operations
    runtime.register_function("unique", Box::new(UniqueFn::new()));
    runtime.register_function("zip", Box::new(ZipFn::new()));
    runtime.register_function("chunk", Box::new(ChunkFn::new()));
    // Note: group_by requires internal interpreter access not exposed by jmespath crate

    // Register custom functions - Object operations
    runtime.register_function("entries", Box::new(EntriesFn::new()));
    runtime.register_function("from_entries", Box::new(FromEntriesFn::new()));

    // Register custom functions - Utility
    runtime.register_function("now", Box::new(NowFn::new()));
    runtime.register_function("default", Box::new(DefaultFn::new()));

    // Register custom functions - Math
    runtime.register_function("round", Box::new(RoundFn::new()));
    runtime.register_function("floor_fn", Box::new(FloorFn::new()));
    runtime.register_function("ceil_fn", Box::new(CeilFn::new()));
    runtime.register_function("abs_fn", Box::new(AbsFn::new()));
    runtime.register_function("mod_fn", Box::new(ModFn::new()));
    runtime.register_function("pow", Box::new(PowFn::new()));
    runtime.register_function("sqrt", Box::new(SqrtFn::new()));
    runtime.register_function("log", Box::new(LogFn::new()));
    runtime.register_function("clamp", Box::new(ClampFn::new()));

    // Register custom functions - More String operations
    runtime.register_function("capitalize", Box::new(CapitalizeFn::new()));
    runtime.register_function("title", Box::new(TitleFn::new()));
    runtime.register_function("repeat", Box::new(RepeatFn::new()));
    runtime.register_function("index_of", Box::new(IndexOfFn::new()));
    runtime.register_function("last_index_of", Box::new(LastIndexOfFn::new()));
    runtime.register_function("slice", Box::new(SliceFn::new()));
    runtime.register_function("concat", Box::new(ConcatFn::new()));

    // Register custom functions - More Array operations
    runtime.register_function("take", Box::new(TakeFn::new()));
    runtime.register_function("drop", Box::new(DropFn::new()));
    runtime.register_function("flatten_deep", Box::new(FlattenDeepFn::new()));
    runtime.register_function("compact", Box::new(CompactFn::new()));
    runtime.register_function("range", Box::new(RangeFn::new()));
    runtime.register_function("index_at", Box::new(IndexAtFn::new()));
    runtime.register_function("includes", Box::new(IncludesFn::new()));
    runtime.register_function("find_index", Box::new(FindIndexFn::new()));

    // Register custom functions - Type conversion/checking
    runtime.register_function("to_string", Box::new(ToStringFn::new()));
    runtime.register_function("to_number", Box::new(ToNumberFn::new()));
    runtime.register_function("to_boolean", Box::new(ToBooleanFn::new()));
    runtime.register_function("type_of", Box::new(TypeOfFn::new()));
    runtime.register_function("is_string", Box::new(IsStringFn::new()));
    runtime.register_function("is_number", Box::new(IsNumberFn::new()));
    runtime.register_function("is_boolean", Box::new(IsBooleanFn::new()));
    runtime.register_function("is_array", Box::new(IsArrayFn::new()));
    runtime.register_function("is_object", Box::new(IsObjectFn::new()));
    runtime.register_function("is_null", Box::new(IsNullFn::new()));

    // Register custom functions - Date/Time
    runtime.register_function("now_ms", Box::new(NowMsFn::new()));

    // Register custom functions - High-impact Tier 1
    runtime.register_function("first", Box::new(FirstFn::new()));
    runtime.register_function("last", Box::new(LastFn::new()));
    runtime.register_function("group_by", Box::new(GroupByFn::new()));
    runtime.register_function("pick", Box::new(PickFn::new()));
    runtime.register_function("omit", Box::new(OmitFn::new()));
    runtime.register_function("difference", Box::new(DifferenceFn::new()));
    runtime.register_function("intersection", Box::new(IntersectionFn::new()));
    runtime.register_function("union", Box::new(UnionFn::new()));
    runtime.register_function("if", Box::new(IfFn::new()));
    runtime.register_function("median", Box::new(MedianFn::new()));
    runtime.register_function("percentile", Box::new(PercentileFn::new()));

    runtime
});

// =============================================================================
// Helper macro for defining functions (similar to jmespath crate's defn!)
// =============================================================================

macro_rules! define_function {
    ($name:ident, $args:expr, $variadic:expr) => {
        pub struct $name {
            signature: Signature,
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $name {
            pub fn new() -> $name {
                $name {
                    signature: Signature::new($args, $variadic),
                }
            }
        }
    };
}

// =============================================================================
// lower(string) -> string
// =============================================================================

define_function!(LowerFn, vec![ArgumentType::String], None);

impl Function for LowerFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        Ok(Rc::new(Variable::String(s.to_lowercase())))
    }
}

// =============================================================================
// upper(string) -> string
// =============================================================================

define_function!(UpperFn, vec![ArgumentType::String], None);

impl Function for UpperFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        Ok(Rc::new(Variable::String(s.to_uppercase())))
    }
}

// =============================================================================
// trim(string) -> string
// =============================================================================

define_function!(TrimFn, vec![ArgumentType::String], None);

impl Function for TrimFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        Ok(Rc::new(Variable::String(s.trim().to_string())))
    }
}

// =============================================================================
// unique(array) -> array
// =============================================================================

define_function!(UniqueFn, vec![ArgumentType::Array], None);

impl Function for UniqueFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        // Use a set to track seen values (by their JSON string representation)
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for item in arr {
            // Use JSON serialization for equality comparison
            let key = serde_json::to_string(&**item).unwrap_or_default();
            if seen.insert(key) {
                result.push(item.clone());
            }
        }

        Ok(Rc::new(Variable::Array(result)))
    }
}

// =============================================================================
// split(string, delimiter) -> array
// =============================================================================

define_function!(
    SplitFn,
    vec![ArgumentType::String, ArgumentType::String],
    None
);

impl Function for SplitFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        let delimiter = args[1].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string delimiter".to_owned()),
            )
        })?;

        let parts: Vec<Rcvar> = s
            .split(delimiter)
            .map(|part| Rc::new(Variable::String(part.to_string())) as Rcvar)
            .collect();

        Ok(Rc::new(Variable::Array(parts)))
    }
}

// =============================================================================
// replace(string, old, new) -> string
// =============================================================================

define_function!(
    ReplaceFn,
    vec![
        ArgumentType::String,
        ArgumentType::String,
        ArgumentType::String
    ],
    None
);

impl Function for ReplaceFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        let old = args[1].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected old string argument".to_owned()),
            )
        })?;

        let new = args[2].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected new string argument".to_owned()),
            )
        })?;

        Ok(Rc::new(Variable::String(s.replace(old, new))))
    }
}

// =============================================================================
// pad_left(string, width, char) -> string
// =============================================================================

define_function!(
    PadLeftFn,
    vec![
        ArgumentType::String,
        ArgumentType::Number,
        ArgumentType::String
    ],
    None
);

impl Function for PadLeftFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        let width = args[1].as_number().map(|n| n as usize).ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected positive number for width".to_owned()),
            )
        })?;

        let pad_char = args[2].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string for pad character".to_owned()),
            )
        })?;

        let pad = pad_char.chars().next().unwrap_or(' ');
        let result = if s.len() >= width {
            s.to_string()
        } else {
            format!("{}{}", pad.to_string().repeat(width - s.len()), s)
        };

        Ok(Rc::new(Variable::String(result)))
    }
}

// =============================================================================
// pad_right(string, width, char) -> string
// =============================================================================

define_function!(
    PadRightFn,
    vec![
        ArgumentType::String,
        ArgumentType::Number,
        ArgumentType::String
    ],
    None
);

impl Function for PadRightFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        let width = args[1].as_number().map(|n| n as usize).ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected positive number for width".to_owned()),
            )
        })?;

        let pad_char = args[2].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string for pad character".to_owned()),
            )
        })?;

        let pad = pad_char.chars().next().unwrap_or(' ');
        let result = if s.len() >= width {
            s.to_string()
        } else {
            format!("{}{}", s, pad.to_string().repeat(width - s.len()))
        };

        Ok(Rc::new(Variable::String(result)))
    }
}

// =============================================================================
// substr(string, start, length?) -> string
// =============================================================================

define_function!(
    SubstrFn,
    vec![ArgumentType::String, ArgumentType::Number],
    Some(ArgumentType::Number)
);

impl Function for SubstrFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        let start = args[1].as_number().map(|n| n as i64).ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected number for start".to_owned()),
            )
        })?;

        // Handle negative start (from end)
        let start_idx = if start < 0 {
            (s.len() as i64 + start).max(0) as usize
        } else {
            start as usize
        };

        let result = if args.len() > 2 {
            let length = args[2].as_number().map(|n| n as usize).ok_or_else(|| {
                JmespathError::new(
                    ctx.expression,
                    0,
                    ErrorReason::Parse("Expected positive number for length".to_owned()),
                )
            })?;
            s.chars().skip(start_idx).take(length).collect()
        } else {
            s.chars().skip(start_idx).collect()
        };

        Ok(Rc::new(Variable::String(result)))
    }
}

// =============================================================================
// now() -> number (Unix timestamp)
// =============================================================================

define_function!(NowFn, vec![], None);

impl Function for NowFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(Rc::new(Variable::Number(serde_json::Number::from(
            timestamp,
        ))))
    }
}

// =============================================================================
// default(value, default_value) -> value if not null, else default
// =============================================================================

define_function!(DefaultFn, vec![ArgumentType::Any, ArgumentType::Any], None);

impl Function for DefaultFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        if args[0].is_null() {
            Ok(args[1].clone())
        } else {
            Ok(args[0].clone())
        }
    }
}

// =============================================================================
// entries(object) -> array of {key, value} objects
// =============================================================================

define_function!(EntriesFn, vec![ArgumentType::Object], None);

impl Function for EntriesFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let obj = args[0].as_object().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected object argument".to_owned()),
            )
        })?;

        let entries: Vec<Rcvar> = obj
            .iter()
            .map(|(k, v)| {
                let mut entry = std::collections::BTreeMap::new();
                entry.insert("key".to_string(), Rc::new(Variable::String(k.clone())));
                entry.insert("value".to_string(), v.clone());
                Rc::new(Variable::Object(entry)) as Rcvar
            })
            .collect();

        Ok(Rc::new(Variable::Array(entries)))
    }
}

// =============================================================================
// from_entries(array) -> object from array of {key, value}
// =============================================================================

define_function!(FromEntriesFn, vec![ArgumentType::Array], None);

impl Function for FromEntriesFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let mut result = std::collections::BTreeMap::new();

        for item in arr {
            if let Some(obj) = item.as_object() {
                if let (Some(key), Some(value)) = (obj.get("key"), obj.get("value")) {
                    if let Some(key_str) = key.as_string() {
                        result.insert(key_str.to_string(), value.clone());
                    }
                }
            }
        }

        Ok(Rc::new(Variable::Object(result)))
    }
}

// =============================================================================
// zip(array1, array2) -> array of pairs
// =============================================================================

define_function!(ZipFn, vec![ArgumentType::Array, ArgumentType::Array], None);

impl Function for ZipFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr1 = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let arr2 = args[1].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let result: Vec<Rcvar> = arr1
            .iter()
            .zip(arr2.iter())
            .map(|(a, b)| Rc::new(Variable::Array(vec![a.clone(), b.clone()])) as Rcvar)
            .collect();

        Ok(Rc::new(Variable::Array(result)))
    }
}

// =============================================================================
// chunk(array, size) -> array of arrays
// =============================================================================

define_function!(
    ChunkFn,
    vec![ArgumentType::Array, ArgumentType::Number],
    None
);

impl Function for ChunkFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let size = args[1].as_number().map(|n| n as usize).ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected positive number for size".to_owned()),
            )
        })?;

        if size == 0 {
            return Ok(Rc::new(Variable::Array(vec![])));
        }

        let chunks: Vec<Rcvar> = arr
            .chunks(size)
            .map(|chunk| Rc::new(Variable::Array(chunk.to_vec())) as Rcvar)
            .collect();

        Ok(Rc::new(Variable::Array(chunks)))
    }
}

// =============================================================================
// MATH FUNCTIONS
// =============================================================================

// =============================================================================
// round(number, precision?) -> number
// =============================================================================

define_function!(
    RoundFn,
    vec![ArgumentType::Number],
    Some(ArgumentType::Number)
);

impl Function for RoundFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let n = args[0].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected number argument".to_owned()),
            )
        })?;

        let precision = if args.len() > 1 {
            args[1].as_number().map(|p| p as i32).unwrap_or(0)
        } else {
            0
        };

        let result = if precision == 0 {
            n.round()
        } else {
            let multiplier = 10_f64.powi(precision);
            (n * multiplier).round() / multiplier
        };

        Ok(Rc::new(Variable::Number(
            serde_json::Number::from_f64(result).unwrap_or_else(|| serde_json::Number::from(0)),
        )))
    }
}

// =============================================================================
// floor_fn(number) -> number
// =============================================================================

define_function!(FloorFn, vec![ArgumentType::Number], None);

impl Function for FloorFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let n = args[0].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected number argument".to_owned()),
            )
        })?;

        Ok(Rc::new(Variable::Number(serde_json::Number::from(
            n.floor() as i64,
        ))))
    }
}

// =============================================================================
// ceil_fn(number) -> number
// =============================================================================

define_function!(CeilFn, vec![ArgumentType::Number], None);

impl Function for CeilFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let n = args[0].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected number argument".to_owned()),
            )
        })?;

        Ok(Rc::new(Variable::Number(serde_json::Number::from(
            n.ceil() as i64,
        ))))
    }
}

// =============================================================================
// abs_fn(number) -> number
// =============================================================================

define_function!(AbsFn, vec![ArgumentType::Number], None);

impl Function for AbsFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let n = args[0].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected number argument".to_owned()),
            )
        })?;

        Ok(Rc::new(Variable::Number(
            serde_json::Number::from_f64(n.abs()).unwrap_or_else(|| serde_json::Number::from(0)),
        )))
    }
}

// =============================================================================
// mod_fn(number, divisor) -> number
// =============================================================================

define_function!(
    ModFn,
    vec![ArgumentType::Number, ArgumentType::Number],
    None
);

impl Function for ModFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let n = args[0].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected number argument".to_owned()),
            )
        })?;

        let divisor = args[1].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected divisor argument".to_owned()),
            )
        })?;

        if divisor == 0.0 {
            return Err(JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Division by zero".to_owned()),
            ));
        }

        Ok(Rc::new(Variable::Number(
            serde_json::Number::from_f64(n % divisor)
                .unwrap_or_else(|| serde_json::Number::from(0)),
        )))
    }
}

// =============================================================================
// pow(base, exponent) -> number
// =============================================================================

define_function!(
    PowFn,
    vec![ArgumentType::Number, ArgumentType::Number],
    None
);

impl Function for PowFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let base = args[0].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected base number".to_owned()),
            )
        })?;

        let exp = args[1].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected exponent number".to_owned()),
            )
        })?;

        Ok(Rc::new(Variable::Number(
            serde_json::Number::from_f64(base.powf(exp))
                .unwrap_or_else(|| serde_json::Number::from(0)),
        )))
    }
}

// =============================================================================
// sqrt(number) -> number
// =============================================================================

define_function!(SqrtFn, vec![ArgumentType::Number], None);

impl Function for SqrtFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let n = args[0].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected number argument".to_owned()),
            )
        })?;

        if n < 0.0 {
            return Err(JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Cannot take square root of negative number".to_owned()),
            ));
        }

        Ok(Rc::new(Variable::Number(
            serde_json::Number::from_f64(n.sqrt()).unwrap_or_else(|| serde_json::Number::from(0)),
        )))
    }
}

// =============================================================================
// log(number, base?) -> number (default base e)
// =============================================================================

define_function!(
    LogFn,
    vec![ArgumentType::Number],
    Some(ArgumentType::Number)
);

impl Function for LogFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let n = args[0].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected number argument".to_owned()),
            )
        })?;

        if n <= 0.0 {
            return Err(JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Logarithm requires positive number".to_owned()),
            ));
        }

        let result = if args.len() > 1 {
            let base = args[1].as_number().ok_or_else(|| {
                JmespathError::new(
                    ctx.expression,
                    0,
                    ErrorReason::Parse("Expected base number".to_owned()),
                )
            })?;
            n.log(base)
        } else {
            n.ln()
        };

        Ok(Rc::new(Variable::Number(
            serde_json::Number::from_f64(result).unwrap_or_else(|| serde_json::Number::from(0)),
        )))
    }
}

// =============================================================================
// clamp(number, min, max) -> number
// =============================================================================

define_function!(
    ClampFn,
    vec![
        ArgumentType::Number,
        ArgumentType::Number,
        ArgumentType::Number
    ],
    None
);

impl Function for ClampFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let n = args[0].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected number argument".to_owned()),
            )
        })?;

        let min = args[1].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected min number".to_owned()),
            )
        })?;

        let max = args[2].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected max number".to_owned()),
            )
        })?;

        let result = n.max(min).min(max);

        Ok(Rc::new(Variable::Number(
            serde_json::Number::from_f64(result).unwrap_or_else(|| serde_json::Number::from(0)),
        )))
    }
}

// =============================================================================
// MORE STRING FUNCTIONS
// =============================================================================

// =============================================================================
// capitalize(string) -> string (first letter uppercase)
// =============================================================================

define_function!(CapitalizeFn, vec![ArgumentType::String], None);

impl Function for CapitalizeFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        let result = if s.is_empty() {
            String::new()
        } else {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        };

        Ok(Rc::new(Variable::String(result)))
    }
}

// =============================================================================
// title(string) -> string (capitalize each word)
// =============================================================================

define_function!(TitleFn, vec![ArgumentType::String], None);

impl Function for TitleFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        let result = s
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().to_string() + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        Ok(Rc::new(Variable::String(result)))
    }
}

// =============================================================================
// repeat(string, count) -> string
// =============================================================================

define_function!(
    RepeatFn,
    vec![ArgumentType::String, ArgumentType::Number],
    None
);

impl Function for RepeatFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        let count = args[1].as_number().map(|n| n as usize).ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected positive number for count".to_owned()),
            )
        })?;

        Ok(Rc::new(Variable::String(s.repeat(count))))
    }
}

// =============================================================================
// index_of(string, search) -> number (-1 if not found)
// =============================================================================

define_function!(
    IndexOfFn,
    vec![ArgumentType::String, ArgumentType::String],
    None
);

impl Function for IndexOfFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        let search = args[1].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected search string".to_owned()),
            )
        })?;

        let result = s.find(search).map(|i| i as i64).unwrap_or(-1);

        Ok(Rc::new(Variable::Number(serde_json::Number::from(result))))
    }
}

// =============================================================================
// last_index_of(string, search) -> number (-1 if not found)
// =============================================================================

define_function!(
    LastIndexOfFn,
    vec![ArgumentType::String, ArgumentType::String],
    None
);

impl Function for LastIndexOfFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        let search = args[1].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected search string".to_owned()),
            )
        })?;

        let result = s.rfind(search).map(|i| i as i64).unwrap_or(-1);

        Ok(Rc::new(Variable::Number(serde_json::Number::from(result))))
    }
}

// =============================================================================
// slice(string, start, end?) -> string
// =============================================================================

define_function!(
    SliceFn,
    vec![ArgumentType::String, ArgumentType::Number],
    Some(ArgumentType::Number)
);

impl Function for SliceFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let s = args[0].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected string argument".to_owned()),
            )
        })?;

        let len = s.len() as i64;

        let start = args[1].as_number().map(|n| n as i64).ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected number for start".to_owned()),
            )
        })?;

        // Handle negative indices
        let start_idx = if start < 0 {
            (len + start).max(0) as usize
        } else {
            start.min(len) as usize
        };

        let end_idx = if args.len() > 2 {
            let end = args[2].as_number().map(|n| n as i64).ok_or_else(|| {
                JmespathError::new(
                    ctx.expression,
                    0,
                    ErrorReason::Parse("Expected number for end".to_owned()),
                )
            })?;
            if end < 0 {
                (len + end).max(0) as usize
            } else {
                end.min(len) as usize
            }
        } else {
            len as usize
        };

        let result: String = s
            .chars()
            .skip(start_idx)
            .take(end_idx.saturating_sub(start_idx))
            .collect();

        Ok(Rc::new(Variable::String(result)))
    }
}

// =============================================================================
// concat(array_of_strings, separator?) -> string
// =============================================================================

define_function!(
    ConcatFn,
    vec![ArgumentType::Array],
    Some(ArgumentType::String)
);

impl Function for ConcatFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let separator = if args.len() > 1 {
            args[1]
                .as_string()
                .map(|s| s.to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let strings: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_string().map(|s| s.to_string()))
            .collect();

        Ok(Rc::new(Variable::String(strings.join(&separator))))
    }
}

// =============================================================================
// MORE ARRAY FUNCTIONS
// =============================================================================

// =============================================================================
// take(array, n) -> array (first n elements)
// =============================================================================

define_function!(
    TakeFn,
    vec![ArgumentType::Array, ArgumentType::Number],
    None
);

impl Function for TakeFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let n = args[1].as_number().map(|n| n as usize).ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected positive number".to_owned()),
            )
        })?;

        let result: Vec<Rcvar> = arr.iter().take(n).cloned().collect();

        Ok(Rc::new(Variable::Array(result)))
    }
}

// =============================================================================
// drop(array, n) -> array (skip first n elements)
// =============================================================================

define_function!(
    DropFn,
    vec![ArgumentType::Array, ArgumentType::Number],
    None
);

impl Function for DropFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let n = args[1].as_number().map(|n| n as usize).ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected positive number".to_owned()),
            )
        })?;

        let result: Vec<Rcvar> = arr.iter().skip(n).cloned().collect();

        Ok(Rc::new(Variable::Array(result)))
    }
}

// =============================================================================
// flatten_deep(array) -> array (recursively flatten)
// =============================================================================

define_function!(FlattenDeepFn, vec![ArgumentType::Array], None);

fn flatten_recursive(arr: &[Rcvar]) -> Vec<Rcvar> {
    let mut result = Vec::new();
    for item in arr {
        if let Some(inner) = item.as_array() {
            result.extend(flatten_recursive(inner));
        } else {
            result.push(item.clone());
        }
    }
    result
}

impl Function for FlattenDeepFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        Ok(Rc::new(Variable::Array(flatten_recursive(arr))))
    }
}

// =============================================================================
// compact(array) -> array (remove null/false values)
// =============================================================================

define_function!(CompactFn, vec![ArgumentType::Array], None);

impl Function for CompactFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let result: Vec<Rcvar> = arr
            .iter()
            .filter(|v| !v.is_null() && !matches!(&***v, Variable::Bool(false)))
            .cloned()
            .collect();

        Ok(Rc::new(Variable::Array(result)))
    }
}

// =============================================================================
// range(start, end, step?) -> array
// =============================================================================

define_function!(
    RangeFn,
    vec![ArgumentType::Number, ArgumentType::Number],
    Some(ArgumentType::Number)
);

impl Function for RangeFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let start = args[0].as_number().map(|n| n as i64).ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected start number".to_owned()),
            )
        })?;

        let end = args[1].as_number().map(|n| n as i64).ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected end number".to_owned()),
            )
        })?;

        let step = if args.len() > 2 {
            args[2].as_number().map(|n| n as i64).ok_or_else(|| {
                JmespathError::new(
                    ctx.expression,
                    0,
                    ErrorReason::Parse("Expected step number".to_owned()),
                )
            })?
        } else {
            1
        };

        if step == 0 {
            return Err(JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Step cannot be zero".to_owned()),
            ));
        }

        let mut result = Vec::new();
        let mut current = start;

        // Limit to prevent runaway allocations
        const MAX_RANGE: usize = 10000;

        if step > 0 {
            while current < end && result.len() < MAX_RANGE {
                result.push(Rc::new(Variable::Number(serde_json::Number::from(current))) as Rcvar);
                current += step;
            }
        } else {
            while current > end && result.len() < MAX_RANGE {
                result.push(Rc::new(Variable::Number(serde_json::Number::from(current))) as Rcvar);
                current += step;
            }
        }

        Ok(Rc::new(Variable::Array(result)))
    }
}

// =============================================================================
// index_at(array, index) -> element (supports negative index)
// =============================================================================

define_function!(
    IndexAtFn,
    vec![ArgumentType::Array, ArgumentType::Number],
    None
);

impl Function for IndexAtFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let index = args[1].as_number().map(|n| n as i64).ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected number for index".to_owned()),
            )
        })?;

        let len = arr.len() as i64;
        let actual_index = if index < 0 {
            (len + index) as usize
        } else {
            index as usize
        };

        if actual_index < arr.len() {
            Ok(arr[actual_index].clone())
        } else {
            Ok(Rc::new(Variable::Null))
        }
    }
}

// =============================================================================
// includes(array, value) -> boolean
// =============================================================================

define_function!(
    IncludesFn,
    vec![ArgumentType::Array, ArgumentType::Any],
    None
);

impl Function for IncludesFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let search_key = serde_json::to_string(&*args[1]).unwrap_or_default();

        let found = arr.iter().any(|item| {
            let item_key = serde_json::to_string(&**item).unwrap_or_default();
            item_key == search_key
        });

        Ok(Rc::new(Variable::Bool(found)))
    }
}

// =============================================================================
// find_index(array, value) -> number (-1 if not found)
// =============================================================================

define_function!(
    FindIndexFn,
    vec![ArgumentType::Array, ArgumentType::Any],
    None
);

impl Function for FindIndexFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let search_key = serde_json::to_string(&*args[1]).unwrap_or_default();

        let index = arr
            .iter()
            .position(|item| {
                let item_key = serde_json::to_string(&**item).unwrap_or_default();
                item_key == search_key
            })
            .map(|i| i as i64)
            .unwrap_or(-1);

        Ok(Rc::new(Variable::Number(serde_json::Number::from(index))))
    }
}

// =============================================================================
// TYPE CONVERSION AND CHECKING FUNCTIONS
// =============================================================================

// =============================================================================
// to_string(any) -> string
// =============================================================================

define_function!(ToStringFn, vec![ArgumentType::Any], None);

impl Function for ToStringFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let result = match &*args[0] {
            Variable::String(s) => s.clone(),
            Variable::Number(n) => n.to_string(),
            Variable::Bool(b) => b.to_string(),
            Variable::Null => "null".to_string(),
            _ => serde_json::to_string(&*args[0]).unwrap_or_else(|_| "null".to_string()),
        };

        Ok(Rc::new(Variable::String(result)))
    }
}

// =============================================================================
// to_number(any) -> number
// =============================================================================

define_function!(ToNumberFn, vec![ArgumentType::Any], None);

impl Function for ToNumberFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let result = match &*args[0] {
            Variable::Number(n) => Some(n.clone()),
            Variable::String(s) => s.parse::<f64>().ok().and_then(serde_json::Number::from_f64),
            Variable::Bool(b) => Some(serde_json::Number::from(if *b { 1 } else { 0 })),
            _ => None,
        };

        match result {
            Some(n) => Ok(Rc::new(Variable::Number(n))),
            None => Ok(Rc::new(Variable::Null)),
        }
    }
}

// =============================================================================
// to_boolean(any) -> boolean
// =============================================================================

define_function!(ToBooleanFn, vec![ArgumentType::Any], None);

impl Function for ToBooleanFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let result = match &*args[0] {
            Variable::Bool(b) => *b,
            Variable::Null => false,
            Variable::String(s) => !s.is_empty(),
            Variable::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
            Variable::Array(a) => !a.is_empty(),
            Variable::Object(o) => !o.is_empty(),
            Variable::Expref(_) => true,
        };

        Ok(Rc::new(Variable::Bool(result)))
    }
}

// =============================================================================
// type_of(any) -> string
// =============================================================================

define_function!(TypeOfFn, vec![ArgumentType::Any], None);

impl Function for TypeOfFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let type_name = match &*args[0] {
            Variable::String(_) => "string",
            Variable::Number(_) => "number",
            Variable::Bool(_) => "boolean",
            Variable::Null => "null",
            Variable::Array(_) => "array",
            Variable::Object(_) => "object",
            Variable::Expref(_) => "expref",
        };

        Ok(Rc::new(Variable::String(type_name.to_string())))
    }
}

// =============================================================================
// is_string(any) -> boolean
// =============================================================================

define_function!(IsStringFn, vec![ArgumentType::Any], None);

impl Function for IsStringFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;
        Ok(Rc::new(Variable::Bool(args[0].is_string())))
    }
}

// =============================================================================
// is_number(any) -> boolean
// =============================================================================

define_function!(IsNumberFn, vec![ArgumentType::Any], None);

impl Function for IsNumberFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;
        Ok(Rc::new(Variable::Bool(matches!(
            &*args[0],
            Variable::Number(_)
        ))))
    }
}

// =============================================================================
// is_boolean(any) -> boolean
// =============================================================================

define_function!(IsBooleanFn, vec![ArgumentType::Any], None);

impl Function for IsBooleanFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;
        Ok(Rc::new(Variable::Bool(matches!(
            &*args[0],
            Variable::Bool(_)
        ))))
    }
}

// =============================================================================
// is_array(any) -> boolean
// =============================================================================

define_function!(IsArrayFn, vec![ArgumentType::Any], None);

impl Function for IsArrayFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;
        Ok(Rc::new(Variable::Bool(args[0].is_array())))
    }
}

// =============================================================================
// is_object(any) -> boolean
// =============================================================================

define_function!(IsObjectFn, vec![ArgumentType::Any], None);

impl Function for IsObjectFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;
        Ok(Rc::new(Variable::Bool(args[0].is_object())))
    }
}

// =============================================================================
// is_null(any) -> boolean
// =============================================================================

define_function!(IsNullFn, vec![ArgumentType::Any], None);

impl Function for IsNullFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;
        Ok(Rc::new(Variable::Bool(args[0].is_null())))
    }
}

// =============================================================================
// now_ms() -> number (Unix timestamp in milliseconds)
// =============================================================================

define_function!(NowMsFn, vec![], None);

impl Function for NowMsFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(Rc::new(Variable::Number(serde_json::Number::from(
            timestamp,
        ))))
    }
}

// =============================================================================
// HIGH-IMPACT FUNCTIONS - Tier 1
// =============================================================================

// =============================================================================
// first(array) -> any (first element or null)
// =============================================================================

define_function!(FirstFn, vec![ArgumentType::Array], None);

impl Function for FirstFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        Ok(arr
            .first()
            .cloned()
            .unwrap_or_else(|| Rc::new(Variable::Null)))
    }
}

// =============================================================================
// last(array) -> any (last element or null)
// =============================================================================

define_function!(LastFn, vec![ArgumentType::Array], None);

impl Function for LastFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        Ok(arr
            .last()
            .cloned()
            .unwrap_or_else(|| Rc::new(Variable::Null)))
    }
}

// =============================================================================
// group_by(array, field_name) -> object (group array of objects by field value)
// =============================================================================

define_function!(
    GroupByFn,
    vec![ArgumentType::Array, ArgumentType::String],
    None
);

impl Function for GroupByFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let field_name = args[1].as_string().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected field name string".to_owned()),
            )
        })?;

        let mut groups: std::collections::BTreeMap<String, Vec<Rcvar>> =
            std::collections::BTreeMap::new();

        for item in arr {
            // Get the field value from the object
            let key = if let Some(obj) = item.as_object() {
                if let Some(field_value) = obj.get(field_name) {
                    match &**field_value {
                        Variable::String(s) => s.clone(),
                        Variable::Number(n) => n.to_string(),
                        Variable::Bool(b) => b.to_string(),
                        Variable::Null => "null".to_string(),
                        _ => continue, // Skip items where key is array/object
                    }
                } else {
                    "null".to_string() // Field doesn't exist
                }
            } else {
                continue; // Skip non-object items
            };
            groups.entry(key).or_default().push(item.clone());
        }

        let result: std::collections::BTreeMap<String, Rcvar> = groups
            .into_iter()
            .map(|(k, v)| (k, Rc::new(Variable::Array(v)) as Rcvar))
            .collect();

        Ok(Rc::new(Variable::Object(result)))
    }
}

// =============================================================================
// pick(object, keys) -> object (select specific keys)
// =============================================================================

define_function!(
    PickFn,
    vec![ArgumentType::Object, ArgumentType::Array],
    None
);

impl Function for PickFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let obj = args[0].as_object().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected object argument".to_owned()),
            )
        })?;

        let keys_arr = args[1].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array of keys".to_owned()),
            )
        })?;

        let keys: HashSet<String> = keys_arr
            .iter()
            .filter_map(|k| k.as_string().map(|s| s.to_string()))
            .collect();

        let result: std::collections::BTreeMap<String, Rcvar> = obj
            .iter()
            .filter(|(k, _)| keys.contains(*k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(Rc::new(Variable::Object(result)))
    }
}

// =============================================================================
// omit(object, keys) -> object (exclude specific keys)
// =============================================================================

define_function!(
    OmitFn,
    vec![ArgumentType::Object, ArgumentType::Array],
    None
);

impl Function for OmitFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let obj = args[0].as_object().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected object argument".to_owned()),
            )
        })?;

        let keys_arr = args[1].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array of keys".to_owned()),
            )
        })?;

        let keys: HashSet<String> = keys_arr
            .iter()
            .filter_map(|k| k.as_string().map(|s| s.to_string()))
            .collect();

        let result: std::collections::BTreeMap<String, Rcvar> = obj
            .iter()
            .filter(|(k, _)| !keys.contains(*k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(Rc::new(Variable::Object(result)))
    }
}

// =============================================================================
// difference(arr1, arr2) -> array (set difference: elements in arr1 not in arr2)
// =============================================================================

define_function!(
    DifferenceFn,
    vec![ArgumentType::Array, ArgumentType::Array],
    None
);

impl Function for DifferenceFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr1 = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let arr2 = args[1].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        // Convert arr2 to a set of JSON strings for comparison
        let set2: HashSet<String> = arr2
            .iter()
            .map(|v| serde_json::to_string(&**v).unwrap_or_default())
            .collect();

        let result: Vec<Rcvar> = arr1
            .iter()
            .filter(|v| {
                let key = serde_json::to_string(&***v).unwrap_or_default();
                !set2.contains(&key)
            })
            .cloned()
            .collect();

        Ok(Rc::new(Variable::Array(result)))
    }
}

// =============================================================================
// intersection(arr1, arr2) -> array (set intersection: elements in both)
// =============================================================================

define_function!(
    IntersectionFn,
    vec![ArgumentType::Array, ArgumentType::Array],
    None
);

impl Function for IntersectionFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr1 = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let arr2 = args[1].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        // Convert arr2 to a set of JSON strings for comparison
        let set2: HashSet<String> = arr2
            .iter()
            .map(|v| serde_json::to_string(&**v).unwrap_or_default())
            .collect();

        let mut seen: HashSet<String> = HashSet::new();
        let result: Vec<Rcvar> = arr1
            .iter()
            .filter(|v| {
                let key = serde_json::to_string(&***v).unwrap_or_default();
                set2.contains(&key) && seen.insert(key)
            })
            .cloned()
            .collect();

        Ok(Rc::new(Variable::Array(result)))
    }
}

// =============================================================================
// union(arr1, arr2) -> array (set union: unique elements from both)
// =============================================================================

define_function!(
    UnionFn,
    vec![ArgumentType::Array, ArgumentType::Array],
    None
);

impl Function for UnionFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr1 = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let arr2 = args[1].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let mut seen: HashSet<String> = HashSet::new();
        let mut result: Vec<Rcvar> = Vec::new();

        for item in arr1.iter().chain(arr2.iter()) {
            let key = serde_json::to_string(&**item).unwrap_or_default();
            if seen.insert(key) {
                result.push(item.clone());
            }
        }

        Ok(Rc::new(Variable::Array(result)))
    }
}

// =============================================================================
// if_fn(condition, then_value, else_value) -> any (ternary conditional)
// Named if_fn to avoid Rust keyword conflict
// =============================================================================

define_function!(
    IfFn,
    vec![ArgumentType::Any, ArgumentType::Any, ArgumentType::Any],
    None
);

impl Function for IfFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let condition = &args[0];
        let then_value = &args[1];
        let else_value = &args[2];

        // JMESPath truthiness: false and null are falsy, everything else is truthy
        let is_truthy = match &**condition {
            Variable::Bool(b) => *b,
            Variable::Null => false,
            _ => true,
        };

        if is_truthy {
            Ok(then_value.clone())
        } else {
            Ok(else_value.clone())
        }
    }
}

// =============================================================================
// median(array) -> number (median value of numeric array)
// =============================================================================

define_function!(MedianFn, vec![ArgumentType::Array], None);

impl Function for MedianFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let mut numbers: Vec<f64> = arr.iter().filter_map(|v| v.as_number()).collect();

        if numbers.is_empty() {
            return Ok(Rc::new(Variable::Null));
        }

        numbers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let len = numbers.len();
        let median = if len % 2 == 0 {
            (numbers[len / 2 - 1] + numbers[len / 2]) / 2.0
        } else {
            numbers[len / 2]
        };

        Ok(Rc::new(Variable::Number(
            serde_json::Number::from_f64(median).unwrap_or_else(|| serde_json::Number::from(0)),
        )))
    }
}

// =============================================================================
// percentile(array, p) -> number (pth percentile, p in 0-100)
// =============================================================================

define_function!(
    PercentileFn,
    vec![ArgumentType::Array, ArgumentType::Number],
    None
);

impl Function for PercentileFn {
    fn evaluate(&self, args: &[Rcvar], ctx: &mut Context<'_>) -> Result<Rcvar, JmespathError> {
        self.signature.validate(args, ctx)?;

        let arr = args[0].as_array().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected array argument".to_owned()),
            )
        })?;

        let p = args[1].as_number().ok_or_else(|| {
            JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Expected percentile value".to_owned()),
            )
        })?;

        if !(0.0..=100.0).contains(&p) {
            return Err(JmespathError::new(
                ctx.expression,
                0,
                ErrorReason::Parse("Percentile must be between 0 and 100".to_owned()),
            ));
        }

        let mut numbers: Vec<f64> = arr.iter().filter_map(|v| v.as_number()).collect();

        if numbers.is_empty() {
            return Ok(Rc::new(Variable::Null));
        }

        numbers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let len = numbers.len();
        if len == 1 {
            return Ok(Rc::new(Variable::Number(
                serde_json::Number::from_f64(numbers[0])
                    .unwrap_or_else(|| serde_json::Number::from(0)),
            )));
        }

        // Linear interpolation method
        let rank = (p / 100.0) * (len - 1) as f64;
        let lower_idx = rank.floor() as usize;
        let upper_idx = rank.ceil() as usize;
        let fraction = rank - lower_idx as f64;

        let result = if lower_idx == upper_idx {
            numbers[lower_idx]
        } else {
            numbers[lower_idx] * (1.0 - fraction) + numbers[upper_idx] * fraction
        };

        Ok(Rc::new(Variable::Number(
            serde_json::Number::from_f64(result).unwrap_or_else(|| serde_json::Number::from(0)),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn query(expr: &str, data: &serde_json::Value) -> Result<String, String> {
        let compiled = REDIS_RUNTIME
            .compile(expr)
            .map_err(|e| format!("Compile error: {e}"))?;
        let result = compiled
            .search(data)
            .map_err(|e| format!("Search error: {e}"))?;
        Ok(serde_json::to_string(&*result).unwrap_or_else(|_| "null".to_string()))
    }

    // =========================================================================
    // lower() tests
    // =========================================================================

    #[test]
    fn test_lower_basic() {
        let data = json!({"name": "ALICE"});
        assert_eq!(query("lower(name)", &data).unwrap(), r#""alice""#);
    }

    #[test]
    fn test_lower_mixed_case() {
        let data = json!({"name": "HeLLo WoRLD"});
        assert_eq!(query("lower(name)", &data).unwrap(), r#""hello world""#);
    }

    #[test]
    fn test_lower_already_lowercase() {
        let data = json!({"name": "already lower"});
        assert_eq!(query("lower(name)", &data).unwrap(), r#""already lower""#);
    }

    #[test]
    fn test_lower_in_filter() {
        let data = json!({"users": [
            {"name": "ALICE"},
            {"name": "Bob"},
            {"name": "CAROL"}
        ]});
        assert_eq!(
            query("users[?lower(name) == 'alice'].name", &data).unwrap(),
            r#"["ALICE"]"#
        );
    }

    // =========================================================================
    // upper() tests
    // =========================================================================

    #[test]
    fn test_upper_basic() {
        let data = json!({"name": "alice"});
        assert_eq!(query("upper(name)", &data).unwrap(), r#""ALICE""#);
    }

    #[test]
    fn test_upper_mixed_case() {
        let data = json!({"name": "HeLLo WoRLD"});
        assert_eq!(query("upper(name)", &data).unwrap(), r#""HELLO WORLD""#);
    }

    // =========================================================================
    // trim() tests
    // =========================================================================

    #[test]
    fn test_trim_basic() {
        let data = json!({"s": "  hello  "});
        assert_eq!(query("trim(s)", &data).unwrap(), r#""hello""#);
    }

    #[test]
    fn test_trim_tabs_and_newlines() {
        let data = json!({"s": "\t\n  hello  \n\t"});
        assert_eq!(query("trim(s)", &data).unwrap(), r#""hello""#);
    }

    #[test]
    fn test_trim_no_whitespace() {
        let data = json!({"s": "hello"});
        assert_eq!(query("trim(s)", &data).unwrap(), r#""hello""#);
    }

    // =========================================================================
    // unique() tests
    // =========================================================================

    #[test]
    fn test_unique_numbers() {
        let data = json!({"nums": [1, 2, 2, 3, 1, 4, 3]});
        assert_eq!(query("unique(nums)", &data).unwrap(), "[1,2,3,4]");
    }

    #[test]
    fn test_unique_strings() {
        let data = json!({"tags": ["a", "b", "a", "c", "b"]});
        assert_eq!(query("unique(tags)", &data).unwrap(), r#"["a","b","c"]"#);
    }

    #[test]
    fn test_unique_preserves_order() {
        let data = json!({"items": ["first", "second", "first", "third", "second"]});
        let result = query("unique(items)", &data).unwrap();
        assert_eq!(result, r#"["first","second","third"]"#);
    }

    #[test]
    fn test_unique_empty_array() {
        let data = json!({"arr": []});
        assert_eq!(query("unique(arr)", &data).unwrap(), "[]");
    }

    #[test]
    fn test_unique_with_projection() {
        let data = json!({"items": [
            {"category": "A"},
            {"category": "B"},
            {"category": "A"},
            {"category": "C"}
        ]});
        assert_eq!(
            query("unique(items[*].category)", &data).unwrap(),
            r#"["A","B","C"]"#
        );
    }

    // =========================================================================
    // split() tests
    // =========================================================================

    #[test]
    fn test_split_basic() {
        let data = json!({"csv": "a,b,c"});
        assert_eq!(query("split(csv, ',')", &data).unwrap(), r#"["a","b","c"]"#);
    }

    #[test]
    fn test_split_spaces() {
        let data = json!({"s": "hello world foo"});
        assert_eq!(
            query("split(s, ' ')", &data).unwrap(),
            r#"["hello","world","foo"]"#
        );
    }

    #[test]
    fn test_split_no_delimiter() {
        let data = json!({"s": "hello"});
        assert_eq!(query("split(s, ',')", &data).unwrap(), r#"["hello"]"#);
    }

    #[test]
    fn test_split_empty_string() {
        let data = json!({"s": ""});
        assert_eq!(query("split(s, ',')", &data).unwrap(), r#"[""]"#);
    }

    #[test]
    fn test_split_multi_char_delimiter() {
        let data = json!({"s": "a::b::c"});
        assert_eq!(query("split(s, '::')", &data).unwrap(), r#"["a","b","c"]"#);
    }

    // =========================================================================
    // replace() tests
    // =========================================================================

    #[test]
    fn test_replace_basic() {
        let data = json!({"s": "hello world"});
        assert_eq!(
            query("replace(s, 'world', 'redis')", &data).unwrap(),
            r#""hello redis""#
        );
    }

    #[test]
    fn test_replace_multiple() {
        let data = json!({"s": "foo bar foo baz foo"});
        assert_eq!(
            query("replace(s, 'foo', 'qux')", &data).unwrap(),
            r#""qux bar qux baz qux""#
        );
    }

    #[test]
    fn test_replace_not_found() {
        let data = json!({"s": "hello"});
        assert_eq!(
            query("replace(s, 'xyz', 'abc')", &data).unwrap(),
            r#""hello""#
        );
    }

    #[test]
    fn test_replace_empty_replacement() {
        let data = json!({"s": "hello world"});
        assert_eq!(
            query("replace(s, ' world', '')", &data).unwrap(),
            r#""hello""#
        );
    }

    // =========================================================================
    // Combined usage tests
    // =========================================================================

    #[test]
    fn test_chained_functions() {
        let data = json!({"s": "  HELLO WORLD  "});
        assert_eq!(query("lower(trim(s))", &data).unwrap(), r#""hello world""#);
    }

    #[test]
    fn test_split_and_unique() {
        let data = json!({"tags": "a,b,a,c,b,d"});
        assert_eq!(
            query("unique(split(tags, ','))", &data).unwrap(),
            r#"["a","b","c","d"]"#
        );
    }

    #[test]
    fn test_complex_pipeline() {
        let data = json!({"items": [
            {"name": "  ALICE  ", "tags": "admin,user"},
            {"name": "  BOB  ", "tags": "user,guest"},
            {"name": "  CAROL  ", "tags": "admin,superuser"}
        ]});

        // Trim and lowercase names
        assert_eq!(
            query("items[*].{name: lower(trim(name))}", &data).unwrap(),
            r#"[{"name":"alice"},{"name":"bob"},{"name":"carol"}]"#
        );
    }

    // =========================================================================
    // MATH FUNCTIONS TESTS
    // =========================================================================

    #[test]
    fn test_round_basic() {
        let data = json!({"n": 3.7});
        assert_eq!(query("round(n)", &data).unwrap(), "4.0");
    }

    #[test]
    fn test_round_with_precision() {
        let data = json!({"n": 3.14159});
        assert_eq!(query("round(n, `2`)", &data).unwrap(), "3.14");
    }

    #[test]
    fn test_round_negative() {
        let data = json!({"n": -2.5});
        // Rust rounds half away from zero, so -2.5 rounds to -3.0
        assert_eq!(query("round(n)", &data).unwrap(), "-3.0");
    }

    #[test]
    fn test_floor_basic() {
        let data = json!({"n": 3.7});
        assert_eq!(query("floor_fn(n)", &data).unwrap(), "3");
    }

    #[test]
    fn test_floor_negative() {
        let data = json!({"n": -2.3});
        assert_eq!(query("floor_fn(n)", &data).unwrap(), "-3");
    }

    #[test]
    fn test_ceil_basic() {
        let data = json!({"n": 3.2});
        assert_eq!(query("ceil_fn(n)", &data).unwrap(), "4");
    }

    #[test]
    fn test_ceil_negative() {
        let data = json!({"n": -2.7});
        assert_eq!(query("ceil_fn(n)", &data).unwrap(), "-2");
    }

    #[test]
    fn test_abs_positive() {
        let data = json!({"n": 5});
        assert_eq!(query("abs_fn(n)", &data).unwrap(), "5.0");
    }

    #[test]
    fn test_abs_negative() {
        let data = json!({"n": -5});
        assert_eq!(query("abs_fn(n)", &data).unwrap(), "5.0");
    }

    #[test]
    fn test_mod_basic() {
        let data = json!({"a": 10, "b": 3});
        assert_eq!(query("mod_fn(a, b)", &data).unwrap(), "1.0");
    }

    #[test]
    fn test_mod_with_float() {
        let data = json!({"a": 10.5, "b": 3});
        assert_eq!(query("mod_fn(a, b)", &data).unwrap(), "1.5");
    }

    #[test]
    fn test_pow_basic() {
        let data = json!({"base": 2, "exp": 3});
        assert_eq!(query("pow(base, exp)", &data).unwrap(), "8.0");
    }

    #[test]
    fn test_pow_fractional() {
        let data = json!({"base": 4, "exp": 0.5});
        assert_eq!(query("pow(base, exp)", &data).unwrap(), "2.0");
    }

    #[test]
    fn test_sqrt_basic() {
        let data = json!({"n": 16});
        assert_eq!(query("sqrt(n)", &data).unwrap(), "4.0");
    }

    #[test]
    fn test_sqrt_non_perfect() {
        let data = json!({"n": 2});
        let result: f64 = query("sqrt(n)", &data).unwrap().parse().unwrap();
        assert!((result - 1.4142135).abs() < 0.0001);
    }

    #[test]
    fn test_log_natural() {
        let data = json!({"n": 2.718281828});
        let result: f64 = query("log(n)", &data).unwrap().parse().unwrap();
        assert!((result - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_log_base_10() {
        let data = json!({"n": 100});
        assert_eq!(query("log(n, `10`)", &data).unwrap(), "2.0");
    }

    #[test]
    fn test_clamp_within_range() {
        let data = json!({"n": 5});
        assert_eq!(query("clamp(n, `0`, `10`)", &data).unwrap(), "5.0");
    }

    #[test]
    fn test_clamp_below_min() {
        let data = json!({"n": -5});
        assert_eq!(query("clamp(n, `0`, `10`)", &data).unwrap(), "0.0");
    }

    #[test]
    fn test_clamp_above_max() {
        let data = json!({"n": 15});
        assert_eq!(query("clamp(n, `0`, `10`)", &data).unwrap(), "10.0");
    }

    // =========================================================================
    // MORE STRING FUNCTIONS TESTS
    // =========================================================================

    #[test]
    fn test_capitalize_basic() {
        let data = json!({"s": "hello"});
        assert_eq!(query("capitalize(s)", &data).unwrap(), r#""Hello""#);
    }

    #[test]
    fn test_capitalize_already_upper() {
        let data = json!({"s": "HELLO"});
        assert_eq!(query("capitalize(s)", &data).unwrap(), r#""HELLO""#);
    }

    #[test]
    fn test_title_basic() {
        let data = json!({"s": "hello world"});
        assert_eq!(query("title(s)", &data).unwrap(), r#""Hello World""#);
    }

    #[test]
    fn test_title_mixed_case() {
        let data = json!({"s": "hELLO wORLD"});
        assert_eq!(query("title(s)", &data).unwrap(), r#""Hello World""#);
    }

    #[test]
    fn test_repeat_basic() {
        let data = json!({"s": "ab"});
        assert_eq!(query("repeat(s, `3`)", &data).unwrap(), r#""ababab""#);
    }

    #[test]
    fn test_repeat_zero() {
        let data = json!({"s": "hello"});
        assert_eq!(query("repeat(s, `0`)", &data).unwrap(), r#""""#);
    }

    #[test]
    fn test_index_of_found() {
        let data = json!({"s": "hello world"});
        assert_eq!(query("index_of(s, 'world')", &data).unwrap(), "6");
    }

    #[test]
    fn test_index_of_not_found() {
        let data = json!({"s": "hello"});
        assert_eq!(query("index_of(s, 'world')", &data).unwrap(), "-1");
    }

    #[test]
    fn test_last_index_of_found() {
        let data = json!({"s": "hello hello"});
        assert_eq!(query("last_index_of(s, 'hello')", &data).unwrap(), "6");
    }

    #[test]
    fn test_slice_basic() {
        let data = json!({"s": "hello world"});
        assert_eq!(query("slice(s, `0`, `5`)", &data).unwrap(), r#""hello""#);
    }

    #[test]
    fn test_slice_negative_start() {
        let data = json!({"s": "hello world"});
        assert_eq!(query("slice(s, `-5`)", &data).unwrap(), r#""world""#);
    }

    #[test]
    fn test_slice_negative_end() {
        let data = json!({"s": "hello world"});
        assert_eq!(query("slice(s, `0`, `-6`)", &data).unwrap(), r#""hello""#);
    }

    #[test]
    fn test_concat_basic() {
        let data = json!({"arr": ["a", "b", "c"]});
        assert_eq!(query("concat(arr)", &data).unwrap(), r#""abc""#);
    }

    #[test]
    fn test_concat_with_separator() {
        let data = json!({"arr": ["a", "b", "c"]});
        assert_eq!(query("concat(arr, '-')", &data).unwrap(), r#""a-b-c""#);
    }

    // =========================================================================
    // MORE ARRAY FUNCTIONS TESTS
    // =========================================================================

    #[test]
    fn test_take_basic() {
        let data = json!({"arr": [1, 2, 3, 4, 5]});
        assert_eq!(query("take(arr, `3`)", &data).unwrap(), "[1,2,3]");
    }

    #[test]
    fn test_take_more_than_length() {
        let data = json!({"arr": [1, 2]});
        assert_eq!(query("take(arr, `5`)", &data).unwrap(), "[1,2]");
    }

    #[test]
    fn test_drop_basic() {
        let data = json!({"arr": [1, 2, 3, 4, 5]});
        assert_eq!(query("drop(arr, `2`)", &data).unwrap(), "[3,4,5]");
    }

    #[test]
    fn test_drop_all() {
        let data = json!({"arr": [1, 2, 3]});
        assert_eq!(query("drop(arr, `5`)", &data).unwrap(), "[]");
    }

    #[test]
    fn test_flatten_deep_nested() {
        let data = json!({"arr": [1, [2, [3, [4]]]]});
        assert_eq!(query("flatten_deep(arr)", &data).unwrap(), "[1,2,3,4]");
    }

    #[test]
    fn test_flatten_deep_already_flat() {
        let data = json!({"arr": [1, 2, 3]});
        assert_eq!(query("flatten_deep(arr)", &data).unwrap(), "[1,2,3]");
    }

    #[test]
    fn test_compact_removes_null() {
        let data = json!({"arr": [1, null, 2, null, 3]});
        assert_eq!(query("compact(arr)", &data).unwrap(), "[1,2,3]");
    }

    #[test]
    fn test_compact_removes_false() {
        let data = json!({"arr": [1, false, 2, true, 3]});
        assert_eq!(query("compact(arr)", &data).unwrap(), "[1,2,true,3]");
    }

    #[test]
    fn test_range_basic() {
        let data = json!({});
        assert_eq!(query("range(`0`, `5`)", &data).unwrap(), "[0,1,2,3,4]");
    }

    #[test]
    fn test_range_with_step() {
        let data = json!({});
        assert_eq!(
            query("range(`0`, `10`, `2`)", &data).unwrap(),
            "[0,2,4,6,8]"
        );
    }

    #[test]
    fn test_range_negative_step() {
        let data = json!({});
        assert_eq!(
            query("range(`5`, `0`, `-1`)", &data).unwrap(),
            "[5,4,3,2,1]"
        );
    }

    #[test]
    fn test_index_at_positive() {
        let data = json!({"arr": ["a", "b", "c", "d"]});
        assert_eq!(query("index_at(arr, `1`)", &data).unwrap(), r#""b""#);
    }

    #[test]
    fn test_index_at_negative() {
        let data = json!({"arr": ["a", "b", "c", "d"]});
        assert_eq!(query("index_at(arr, `-1`)", &data).unwrap(), r#""d""#);
    }

    #[test]
    fn test_index_at_out_of_bounds() {
        let data = json!({"arr": ["a", "b"]});
        assert_eq!(query("index_at(arr, `10`)", &data).unwrap(), "null");
    }

    #[test]
    fn test_includes_found() {
        let data = json!({"arr": [1, 2, 3]});
        assert_eq!(query("includes(arr, `2`)", &data).unwrap(), "true");
    }

    #[test]
    fn test_includes_not_found() {
        let data = json!({"arr": [1, 2, 3]});
        assert_eq!(query("includes(arr, `5`)", &data).unwrap(), "false");
    }

    #[test]
    fn test_includes_string() {
        let data = json!({"arr": ["a", "b", "c"]});
        assert_eq!(query("includes(arr, 'b')", &data).unwrap(), "true");
    }

    #[test]
    fn test_find_index_found() {
        let data = json!({"arr": ["a", "b", "c"]});
        assert_eq!(query("find_index(arr, 'b')", &data).unwrap(), "1");
    }

    #[test]
    fn test_find_index_not_found() {
        let data = json!({"arr": ["a", "b", "c"]});
        assert_eq!(query("find_index(arr, 'd')", &data).unwrap(), "-1");
    }

    // =========================================================================
    // TYPE FUNCTIONS TESTS
    // =========================================================================

    #[test]
    fn test_to_string_number() {
        let data = json!({"n": 42});
        assert_eq!(query("to_string(n)", &data).unwrap(), r#""42""#);
    }

    #[test]
    fn test_to_string_bool() {
        let data = json!({"b": true});
        assert_eq!(query("to_string(b)", &data).unwrap(), r#""true""#);
    }

    #[test]
    fn test_to_number_string() {
        let data = json!({"s": "42"});
        assert_eq!(query("to_number(s)", &data).unwrap(), "42.0");
    }

    #[test]
    fn test_to_number_invalid() {
        let data = json!({"s": "hello"});
        assert_eq!(query("to_number(s)", &data).unwrap(), "null");
    }

    #[test]
    fn test_to_boolean_string() {
        let data = json!({"s": "hello"});
        assert_eq!(query("to_boolean(s)", &data).unwrap(), "true");
    }

    #[test]
    fn test_to_boolean_empty_string() {
        let data = json!({"s": ""});
        assert_eq!(query("to_boolean(s)", &data).unwrap(), "false");
    }

    #[test]
    fn test_to_boolean_null() {
        let data = json!({"n": null});
        assert_eq!(query("to_boolean(n)", &data).unwrap(), "false");
    }

    #[test]
    fn test_type_of_string() {
        let data = json!({"s": "hello"});
        assert_eq!(query("type_of(s)", &data).unwrap(), r#""string""#);
    }

    #[test]
    fn test_type_of_number() {
        let data = json!({"n": 42});
        assert_eq!(query("type_of(n)", &data).unwrap(), r#""number""#);
    }

    #[test]
    fn test_type_of_boolean() {
        let data = json!({"b": true});
        assert_eq!(query("type_of(b)", &data).unwrap(), r#""boolean""#);
    }

    #[test]
    fn test_type_of_array() {
        let data = json!({"a": [1, 2, 3]});
        assert_eq!(query("type_of(a)", &data).unwrap(), r#""array""#);
    }

    #[test]
    fn test_type_of_object() {
        let data = json!({"o": {"key": "value"}});
        assert_eq!(query("type_of(o)", &data).unwrap(), r#""object""#);
    }

    #[test]
    fn test_type_of_null() {
        let data = json!({"n": null});
        assert_eq!(query("type_of(n)", &data).unwrap(), r#""null""#);
    }

    #[test]
    fn test_is_string_true() {
        let data = json!({"s": "hello"});
        assert_eq!(query("is_string(s)", &data).unwrap(), "true");
    }

    #[test]
    fn test_is_string_false() {
        let data = json!({"n": 42});
        assert_eq!(query("is_string(n)", &data).unwrap(), "false");
    }

    #[test]
    fn test_is_number_true() {
        let data = json!({"n": 42});
        assert_eq!(query("is_number(n)", &data).unwrap(), "true");
    }

    #[test]
    fn test_is_number_false() {
        let data = json!({"s": "hello"});
        assert_eq!(query("is_number(s)", &data).unwrap(), "false");
    }

    #[test]
    fn test_is_boolean_true() {
        let data = json!({"b": true});
        assert_eq!(query("is_boolean(b)", &data).unwrap(), "true");
    }

    #[test]
    fn test_is_array_true() {
        let data = json!({"a": [1, 2, 3]});
        assert_eq!(query("is_array(a)", &data).unwrap(), "true");
    }

    #[test]
    fn test_is_object_true() {
        let data = json!({"o": {"key": "value"}});
        assert_eq!(query("is_object(o)", &data).unwrap(), "true");
    }

    #[test]
    fn test_is_null_true() {
        let data = json!({"n": null});
        assert_eq!(query("is_null(n)", &data).unwrap(), "true");
    }

    #[test]
    fn test_is_null_false() {
        let data = json!({"s": "hello"});
        assert_eq!(query("is_null(s)", &data).unwrap(), "false");
    }

    // =========================================================================
    // now() and now_ms() tests
    // =========================================================================

    #[test]
    fn test_now_returns_timestamp() {
        let data = json!({});
        let result: u64 = query("now()", &data).unwrap().parse().unwrap();
        // Should be a reasonable Unix timestamp (after 2020)
        assert!(result > 1577836800);
    }

    #[test]
    fn test_now_ms_returns_timestamp() {
        let data = json!({});
        let result: u64 = query("now_ms()", &data).unwrap().parse().unwrap();
        // Should be a reasonable Unix timestamp in ms (after 2020)
        assert!(result > 1577836800000);
    }

    // =========================================================================
    // default() tests
    // =========================================================================

    #[test]
    fn test_default_with_value() {
        let data = json!({"name": "Alice"});
        assert_eq!(
            query("default(name, 'Unknown')", &data).unwrap(),
            r#""Alice""#
        );
    }

    #[test]
    fn test_default_with_null() {
        let data = json!({"name": null});
        assert_eq!(
            query("default(name, 'Unknown')", &data).unwrap(),
            r#""Unknown""#
        );
    }

    #[test]
    fn test_default_with_missing() {
        let data = json!({});
        assert_eq!(
            query("default(name, 'Unknown')", &data).unwrap(),
            r#""Unknown""#
        );
    }

    // =========================================================================
    // entries() and from_entries() tests
    // =========================================================================

    #[test]
    fn test_entries_basic() {
        let data = json!({"obj": {"a": 1, "b": 2}});
        let result = query("entries(obj)", &data).unwrap();
        // BTreeMap maintains order, so this should be consistent
        assert!(result.contains(r#""key":"a""#));
        assert!(result.contains(r#""value":1"#));
    }

    #[test]
    fn test_from_entries_basic() {
        let data = json!({"arr": [{"key": "a", "value": 1}, {"key": "b", "value": 2}]});
        let result = query("from_entries(arr)", &data).unwrap();
        assert!(result.contains(r#""a":1"#));
        assert!(result.contains(r#""b":2"#));
    }

    #[test]
    fn test_entries_from_entries_roundtrip() {
        let data = json!({"obj": {"x": 10, "y": 20}});
        let result = query("from_entries(entries(obj))", &data).unwrap();
        assert!(result.contains(r#""x":10"#));
        assert!(result.contains(r#""y":20"#));
    }

    // =========================================================================
    // zip() tests
    // =========================================================================

    #[test]
    fn test_zip_equal_length() {
        let data = json!({"a": [1, 2, 3], "b": ["a", "b", "c"]});
        assert_eq!(
            query("zip(a, b)", &data).unwrap(),
            r#"[[1,"a"],[2,"b"],[3,"c"]]"#
        );
    }

    #[test]
    fn test_zip_different_length() {
        let data = json!({"a": [1, 2], "b": ["a", "b", "c"]});
        assert_eq!(query("zip(a, b)", &data).unwrap(), r#"[[1,"a"],[2,"b"]]"#);
    }

    // =========================================================================
    // chunk() tests
    // =========================================================================

    #[test]
    fn test_chunk_basic() {
        let data = json!({"arr": [1, 2, 3, 4, 5]});
        assert_eq!(
            query("chunk(arr, `2`)", &data).unwrap(),
            "[[1,2],[3,4],[5]]"
        );
    }

    #[test]
    fn test_chunk_exact() {
        let data = json!({"arr": [1, 2, 3, 4]});
        assert_eq!(query("chunk(arr, `2`)", &data).unwrap(), "[[1,2],[3,4]]");
    }

    // =========================================================================
    // pad_left() and pad_right() tests
    // =========================================================================

    #[test]
    fn test_pad_left_basic() {
        let data = json!({"s": "42"});
        assert_eq!(query("pad_left(s, `5`, '0')", &data).unwrap(), r#""00042""#);
    }

    #[test]
    fn test_pad_left_no_padding_needed() {
        let data = json!({"s": "hello"});
        assert_eq!(query("pad_left(s, `3`, ' ')", &data).unwrap(), r#""hello""#);
    }

    #[test]
    fn test_pad_right_basic() {
        let data = json!({"s": "hi"});
        assert_eq!(
            query("pad_right(s, `5`, '-')", &data).unwrap(),
            r#""hi---""#
        );
    }

    // =========================================================================
    // substr() tests
    // =========================================================================

    #[test]
    fn test_substr_basic() {
        let data = json!({"s": "hello world"});
        assert_eq!(query("substr(s, `0`, `5`)", &data).unwrap(), r#""hello""#);
    }

    #[test]
    fn test_substr_negative_start() {
        let data = json!({"s": "hello world"});
        assert_eq!(query("substr(s, `-5`, `5`)", &data).unwrap(), r#""world""#);
    }

    #[test]
    fn test_substr_no_length() {
        let data = json!({"s": "hello world"});
        assert_eq!(query("substr(s, `6`)", &data).unwrap(), r#""world""#);
    }

    // =========================================================================
    // Complex combined tests
    // =========================================================================

    #[test]
    fn test_math_in_filter() {
        let data = json!({"items": [
            {"price": 10.5},
            {"price": 20.3},
            {"price": 15.7}
        ]});
        assert_eq!(
            query("items[?round(price) > `15`].price", &data).unwrap(),
            "[20.3,15.7]"
        );
    }

    #[test]
    fn test_type_checking_filter() {
        let data = json!({"items": [1, "hello", true, null, [1,2], {"a": 1}]});
        assert_eq!(query("items[?is_number(@)]", &data).unwrap(), "[1]");
    }

    #[test]
    fn test_string_processing_pipeline() {
        let data = json!({"names": ["  alice  ", "  BOB  ", "  Carol  "]});
        assert_eq!(
            query("names[*] | map(&title(trim(@)), @)", &data).unwrap(),
            r#"["Alice","Bob","Carol"]"#
        );
    }

    // =========================================================================
    // first() tests
    // =========================================================================

    #[test]
    fn test_first_basic() {
        let data = json!({"items": [1, 2, 3]});
        assert_eq!(query("first(items)", &data).unwrap(), "1");
    }

    #[test]
    fn test_first_empty_array() {
        let data = json!({"items": []});
        assert_eq!(query("first(items)", &data).unwrap(), "null");
    }

    #[test]
    fn test_first_strings() {
        let data = json!({"items": ["a", "b", "c"]});
        assert_eq!(query("first(items)", &data).unwrap(), r#""a""#);
    }

    #[test]
    fn test_first_objects() {
        let data = json!({"items": [{"id": 1}, {"id": 2}]});
        assert_eq!(query("first(items)", &data).unwrap(), r#"{"id":1}"#);
    }

    // =========================================================================
    // last() tests
    // =========================================================================

    #[test]
    fn test_last_basic() {
        let data = json!({"items": [1, 2, 3]});
        assert_eq!(query("last(items)", &data).unwrap(), "3");
    }

    #[test]
    fn test_last_empty_array() {
        let data = json!({"items": []});
        assert_eq!(query("last(items)", &data).unwrap(), "null");
    }

    #[test]
    fn test_last_strings() {
        let data = json!({"items": ["a", "b", "c"]});
        assert_eq!(query("last(items)", &data).unwrap(), r#""c""#);
    }

    #[test]
    fn test_last_single_element() {
        let data = json!({"items": [42]});
        assert_eq!(query("last(items)", &data).unwrap(), "42");
    }

    // =========================================================================
    // group_by() tests
    // =========================================================================

    #[test]
    fn test_group_by_basic() {
        let data = json!([
            {"name": "Alice", "role": "admin"},
            {"name": "Bob", "role": "user"},
            {"name": "Carol", "role": "admin"}
        ]);
        let result = query("group_by(@, 'role')", &data).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["admin"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["user"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_group_by_numbers() {
        let data = json!([
            {"name": "A", "score": 10},
            {"name": "B", "score": 20},
            {"name": "C", "score": 10}
        ]);
        let result = query("group_by(@, 'score')", &data).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["10"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["20"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_group_by_missing_field() {
        let data = json!([
            {"name": "Alice", "role": "admin"},
            {"name": "Bob"}
        ]);
        let result = query("group_by(@, 'role')", &data).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["admin"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["null"].as_array().unwrap().len(), 1);
    }

    // =========================================================================
    // pick() tests
    // =========================================================================

    #[test]
    fn test_pick_basic() {
        let data = json!({"name": "Alice", "age": 30, "email": "alice@example.com"});
        let result = query("pick(@, ['name', 'age'])", &data).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["age"], 30);
        assert!(parsed.get("email").is_none());
    }

    #[test]
    fn test_pick_nonexistent_keys() {
        let data = json!({"name": "Alice", "age": 30});
        let result = query("pick(@, ['name', 'missing'])", &data).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["name"], "Alice");
        assert!(parsed.get("missing").is_none());
    }

    #[test]
    fn test_pick_empty_keys() {
        let data = json!({"name": "Alice", "age": 30, "keys": []});
        assert_eq!(query("pick(@, keys)", &data).unwrap(), "{}");
    }

    // =========================================================================
    // omit() tests
    // =========================================================================

    #[test]
    fn test_omit_basic() {
        let data = json!({"name": "Alice", "age": 30, "password": "secret"});
        let result = query("omit(@, ['password'])", &data).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["age"], 30);
        assert!(parsed.get("password").is_none());
    }

    #[test]
    fn test_omit_multiple_keys() {
        let data = json!({"a": 1, "b": 2, "c": 3, "d": 4});
        let result = query("omit(@, ['b', 'd'])", &data).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["a"], 1);
        assert_eq!(parsed["c"], 3);
        assert!(parsed.get("b").is_none());
        assert!(parsed.get("d").is_none());
    }

    #[test]
    fn test_omit_nonexistent_keys() {
        let data = json!({"name": "Alice"});
        let result = query("omit(@, ['missing'])", &data).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["name"], "Alice");
    }

    // =========================================================================
    // difference() tests
    // =========================================================================

    #[test]
    fn test_difference_basic() {
        let data = json!({"a": [1, 2, 3, 4], "b": [2, 4]});
        assert_eq!(query("difference(a, b)", &data).unwrap(), "[1,3]");
    }

    #[test]
    fn test_difference_strings() {
        let data = json!({"a": ["x", "y", "z"], "b": ["y"]});
        assert_eq!(query("difference(a, b)", &data).unwrap(), r#"["x","z"]"#);
    }

    #[test]
    fn test_difference_no_overlap() {
        let data = json!({"a": [1, 2], "b": [3, 4]});
        assert_eq!(query("difference(a, b)", &data).unwrap(), "[1,2]");
    }

    #[test]
    fn test_difference_complete_overlap() {
        let data = json!({"a": [1, 2], "b": [1, 2]});
        assert_eq!(query("difference(a, b)", &data).unwrap(), "[]");
    }

    // =========================================================================
    // intersection() tests
    // =========================================================================

    #[test]
    fn test_intersection_basic() {
        let data = json!({"a": [1, 2, 3], "b": [2, 3, 4]});
        assert_eq!(query("intersection(a, b)", &data).unwrap(), "[2,3]");
    }

    #[test]
    fn test_intersection_strings() {
        let data = json!({"a": ["x", "y", "z"], "b": ["y", "z", "w"]});
        assert_eq!(query("intersection(a, b)", &data).unwrap(), r#"["y","z"]"#);
    }

    #[test]
    fn test_intersection_no_overlap() {
        let data = json!({"a": [1, 2], "b": [3, 4]});
        assert_eq!(query("intersection(a, b)", &data).unwrap(), "[]");
    }

    #[test]
    fn test_intersection_duplicates() {
        let data = json!({"a": [1, 1, 2, 2], "b": [1, 2, 2]});
        // Should return unique intersection
        assert_eq!(query("intersection(a, b)", &data).unwrap(), "[1,2]");
    }

    // =========================================================================
    // union() tests
    // =========================================================================

    #[test]
    fn test_union_basic() {
        let data = json!({"a": [1, 2], "b": [2, 3]});
        assert_eq!(query("union(a, b)", &data).unwrap(), "[1,2,3]");
    }

    #[test]
    fn test_union_strings() {
        let data = json!({"a": ["x", "y"], "b": ["y", "z"]});
        assert_eq!(query("union(a, b)", &data).unwrap(), r#"["x","y","z"]"#);
    }

    #[test]
    fn test_union_no_overlap() {
        let data = json!({"a": [1, 2], "b": [3, 4]});
        assert_eq!(query("union(a, b)", &data).unwrap(), "[1,2,3,4]");
    }

    #[test]
    fn test_union_complete_overlap() {
        let data = json!({"a": [1, 2], "b": [1, 2]});
        assert_eq!(query("union(a, b)", &data).unwrap(), "[1,2]");
    }

    // =========================================================================
    // if() tests
    // =========================================================================

    #[test]
    fn test_if_true_condition() {
        let data = json!({"active": true});
        assert_eq!(query("if(active, 'yes', 'no')", &data).unwrap(), r#""yes""#);
    }

    #[test]
    fn test_if_false_condition() {
        let data = json!({"active": false});
        assert_eq!(query("if(active, 'yes', 'no')", &data).unwrap(), r#""no""#);
    }

    #[test]
    fn test_if_null_is_falsy() {
        let data = json!({"value": null});
        assert_eq!(
            query("if(value, 'has value', 'empty')", &data).unwrap(),
            r#""empty""#
        );
    }

    #[test]
    fn test_if_string_is_truthy() {
        let data = json!({"name": "Alice"});
        assert_eq!(
            query("if(name, 'has name', 'no name')", &data).unwrap(),
            r#""has name""#
        );
    }

    #[test]
    fn test_if_number_is_truthy() {
        let data = json!({"count": 0});
        // 0 is truthy in JMESPath (only false and null are falsy)
        assert_eq!(
            query("if(count, 'has count', 'no count')", &data).unwrap(),
            r#""has count""#
        );
    }

    #[test]
    fn test_if_with_comparison() {
        let data = json!({"age": 25});
        assert_eq!(
            query("if(age > `18`, 'adult', 'minor')", &data).unwrap(),
            r#""adult""#
        );
    }

    #[test]
    fn test_if_nested() {
        let data = json!({"score": 85});
        assert_eq!(
            query("if(score >= `90`, 'A', if(score >= `80`, 'B', 'C'))", &data).unwrap(),
            r#""B""#
        );
    }

    // =========================================================================
    // median() tests
    // =========================================================================

    #[test]
    fn test_median_odd_count() {
        let data = json!({"nums": [1, 3, 5, 7, 9]});
        assert_eq!(query("median(nums)", &data).unwrap(), "5.0");
    }

    #[test]
    fn test_median_even_count() {
        let data = json!({"nums": [1, 2, 3, 4]});
        assert_eq!(query("median(nums)", &data).unwrap(), "2.5");
    }

    #[test]
    fn test_median_single_element() {
        let data = json!({"nums": [42]});
        assert_eq!(query("median(nums)", &data).unwrap(), "42.0");
    }

    #[test]
    fn test_median_unsorted() {
        let data = json!({"nums": [5, 1, 9, 3, 7]});
        assert_eq!(query("median(nums)", &data).unwrap(), "5.0");
    }

    #[test]
    fn test_median_empty() {
        let data = json!({"nums": []});
        assert_eq!(query("median(nums)", &data).unwrap(), "null");
    }

    #[test]
    fn test_median_with_floats() {
        let data = json!({"nums": [1.5, 2.5, 3.5]});
        assert_eq!(query("median(nums)", &data).unwrap(), "2.5");
    }

    // =========================================================================
    // percentile() tests
    // =========================================================================

    #[test]
    fn test_percentile_50th() {
        let data = json!({"nums": [1, 2, 3, 4, 5]});
        // 50th percentile is median
        assert_eq!(query("percentile(nums, `50`)", &data).unwrap(), "3.0");
    }

    #[test]
    fn test_percentile_0th() {
        let data = json!({"nums": [1, 2, 3, 4, 5]});
        assert_eq!(query("percentile(nums, `0`)", &data).unwrap(), "1.0");
    }

    #[test]
    fn test_percentile_100th() {
        let data = json!({"nums": [1, 2, 3, 4, 5]});
        assert_eq!(query("percentile(nums, `100`)", &data).unwrap(), "5.0");
    }

    #[test]
    fn test_percentile_25th() {
        let data = json!({"nums": [1, 2, 3, 4, 5]});
        // Linear interpolation: rank = 0.25 * 4 = 1.0, so index 1 = 2.0
        assert_eq!(query("percentile(nums, `25`)", &data).unwrap(), "2.0");
    }

    #[test]
    fn test_percentile_75th() {
        let data = json!({"nums": [1, 2, 3, 4, 5]});
        // Linear interpolation: rank = 0.75 * 4 = 3.0, so index 3 = 4.0
        assert_eq!(query("percentile(nums, `75`)", &data).unwrap(), "4.0");
    }

    #[test]
    fn test_percentile_95th() {
        let data = json!({"nums": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]});
        // rank = 0.95 * 9 = 8.55, interpolate between index 8 (9) and 9 (10)
        // 9 * 0.45 + 10 * 0.55 = 4.05 + 5.5 = 9.55
        let result = query("percentile(nums, `95`)", &data).unwrap();
        let value: f64 = result.parse().unwrap();
        assert!((value - 9.55).abs() < 0.01);
    }

    #[test]
    fn test_percentile_empty() {
        let data = json!({"nums": []});
        assert_eq!(query("percentile(nums, `50`)", &data).unwrap(), "null");
    }

    #[test]
    fn test_percentile_single_element() {
        let data = json!({"nums": [42]});
        assert_eq!(query("percentile(nums, `99`)", &data).unwrap(), "42.0");
    }
}
