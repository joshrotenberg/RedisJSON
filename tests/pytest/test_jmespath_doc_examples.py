# -*- coding: utf-8 -*-

"""
Tests for JMESPath examples from documentation.

This file tests examples from docs/jmespath.md to ensure documentation
accuracy. Each test corresponds to documented function behavior.
"""

import json

from RLTest import Defaults, Env

Defaults.decode_responses = True


# -----------------------------------------------------------------------------
# String Functions
# -----------------------------------------------------------------------------


def testDocExamples_lower(env):
    """Test lower() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"name": "ALICE"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "lower(name)")
    r.assertEqual(result, '"alice"')


def testDocExamples_upper(env):
    """Test upper() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"name": "alice"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "upper(name)")
    r.assertEqual(result, '"ALICE"')


def testDocExamples_trim(env):
    """Test trim() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"input": "  hello world  "}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "trim(input)")
    r.assertEqual(result, '"hello world"')


def testDocExamples_capitalize(env):
    """Test capitalize() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"name": "alice"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "capitalize(name)")
    r.assertEqual(result, '"Alice"')


def testDocExamples_title(env):
    """Test title() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"title": "hello world"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "title(title)")
    r.assertEqual(result, '"Hello World"')


def testDocExamples_split(env):
    """Test split() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"tags": "redis,json,database"}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "split(tags, ',')")
    r.assertEqual(json.loads(result), ["redis", "json", "database"])


def testDocExamples_replace(env):
    """Test replace() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"text": "hello world"}'))
    result = r.execute_command(
        "JSON.JMESPATH", "doc", "replace(text, 'world', 'redis')"
    )
    r.assertEqual(result, '"hello redis"')


def testDocExamples_repeat(env):
    """Test repeat() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "ab"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "repeat(s, `3`)")
    r.assertEqual(result, '"ababab"')


def testDocExamples_pad_left(env):
    """Test pad_left() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"id": "42"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "pad_left(id, `5`, '0')")
    r.assertEqual(result, '"00042"')


def testDocExamples_pad_right(env):
    """Test pad_right() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"id": "42"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "pad_right(id, `5`, '-')")
    r.assertEqual(result, '"42---"')


def testDocExamples_substr(env):
    """Test substr() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "hello world"}'))

    # Positive indices
    result = r.execute_command("JSON.JMESPATH", "doc", "substr(s, `0`, `5`)")
    r.assertEqual(result, '"hello"')

    # Negative index
    result = r.execute_command("JSON.JMESPATH", "doc", "substr(s, `-5`)")
    r.assertEqual(result, '"world"')


def testDocExamples_slice(env):
    """Test slice() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "hello world"}'))

    result = r.execute_command("JSON.JMESPATH", "doc", "slice(s, `0`, `5`)")
    r.assertEqual(result, '"hello"')

    result = r.execute_command("JSON.JMESPATH", "doc", "slice(s, `-5`)")
    r.assertEqual(result, '"world"')


def testDocExamples_index_of(env):
    """Test index_of() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "hello world"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "index_of(s, 'world')")
    r.assertEqual(result, "6")


def testDocExamples_last_index_of(env):
    """Test last_index_of() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "hello hello"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "last_index_of(s, 'hello')")
    r.assertEqual(result, "6")


def testDocExamples_concat(env):
    """Test concat() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"parts": ["a", "b", "c"]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "concat(parts, '-')")
    r.assertEqual(result, '"a-b-c"')


def testDocExamples_truncate(env):
    """Test truncate() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "hello world"}'))

    result = r.execute_command("JSON.JMESPATH", "doc", "truncate(s, `5`)")
    r.assertEqual(result, '"he..."')

    # No truncation needed
    result = r.execute_command("JSON.JMESPATH", "doc", "truncate(s, `50`)")
    r.assertEqual(result, '"hello world"')


def testDocExamples_trim_start(env):
    """Test trim_start() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "   hello"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "trim_start(s)")
    r.assertEqual(result, '"hello"')


def testDocExamples_trim_end(env):
    """Test trim_end() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "hello   "}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "trim_end(s)")
    r.assertEqual(result, '"hello"')


def testDocExamples_format(env):
    """Test format() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"name": "Alice", "age": 30}')
    )
    result = r.execute_command(
        "JSON.JMESPATH", "doc", "format('{0} is {1} years old', name, age)"
    )
    r.assertEqual(result, '"Alice is 30 years old"')


def testDocExamples_camel_case(env):
    """Test camel_case() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "hello_world"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "camel_case(s)")
    r.assertEqual(result, '"helloWorld"')


def testDocExamples_snake_case(env):
    """Test snake_case() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "helloWorld"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "snake_case(s)")
    r.assertEqual(result, '"hello_world"')


def testDocExamples_kebab_case(env):
    """Test kebab_case() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "helloWorld"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "kebab_case(s)")
    r.assertEqual(result, '"hello-world"')


# -----------------------------------------------------------------------------
# Regex Functions
# -----------------------------------------------------------------------------


def testDocExamples_regex_match(env):
    """Test regex_match() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"email": "user@example.com"}')
    )
    result = r.execute_command(
        "JSON.JMESPATH", "doc", "regex_match(email, '^[^@]+@[^@]+\\\\.[^@]+$')"
    )
    r.assertEqual(result, "true")


def testDocExamples_regex_extract(env):
    """Test regex_extract() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET", "doc", "$", '{"text": "Call 555-1234 or 555-5678"}'
        )
    )
    result = r.execute_command(
        "JSON.JMESPATH", "doc", "regex_extract(text, '\\\\d{3}-\\\\d{4}')"
    )
    r.assertEqual(json.loads(result), ["555-1234", "555-5678"])


def testDocExamples_regex_replace(env):
    """Test regex_replace() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"text": "Hello World"}'))
    result = r.execute_command(
        "JSON.JMESPATH", "doc", "regex_replace(text, 'World', 'Redis')"
    )
    r.assertEqual(result, '"Hello Redis"')


# -----------------------------------------------------------------------------
# Array Functions
# -----------------------------------------------------------------------------


def testDocExamples_unique(env):
    """Test unique() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"tags": ["a", "b", "a", "c", "b"]}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "unique(tags)")
    r.assertEqual(json.loads(result), ["a", "b", "c"])


def testDocExamples_zip(env):
    """Test zip() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET", "doc", "$", '{"keys": ["a","b","c"], "vals": [1,2,3]}'
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "zip(keys, vals)")
    r.assertEqual(json.loads(result), [["a", 1], ["b", 2], ["c", 3]])


def testDocExamples_chunk(env):
    """Test chunk() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"items": [1,2,3,4,5,6,7]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "chunk(items, `3`)")
    r.assertEqual(json.loads(result), [[1, 2, 3], [4, 5, 6], [7]])


def testDocExamples_take(env):
    """Test take() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"arr": [1, 2, 3, 4, 5]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "take(arr, `3`)")
    r.assertEqual(json.loads(result), [1, 2, 3])


def testDocExamples_drop(env):
    """Test drop() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"arr": [1, 2, 3, 4, 5]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "drop(arr, `2`)")
    r.assertEqual(json.loads(result), [3, 4, 5])


def testDocExamples_flatten_deep(env):
    """Test flatten_deep() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"arr": [1, [2, [3, [4]]]]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "flatten_deep(arr)")
    r.assertEqual(json.loads(result), [1, 2, 3, 4])


def testDocExamples_compact(env):
    """Test compact() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"arr": [1, null, 2, false, 3]}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "compact(arr)")
    r.assertEqual(json.loads(result), [1, 2, 3])


def testDocExamples_range(env):
    """Test range() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", "{}"))

    result = r.execute_command("JSON.JMESPATH", "doc", "range(`0`, `5`)")
    r.assertEqual(json.loads(result), [0, 1, 2, 3, 4])

    result = r.execute_command("JSON.JMESPATH", "doc", "range(`0`, `10`, `2`)")
    r.assertEqual(json.loads(result), [0, 2, 4, 6, 8])

    result = r.execute_command("JSON.JMESPATH", "doc", "range(`5`, `0`, `-1`)")
    r.assertEqual(json.loads(result), [5, 4, 3, 2, 1])


def testDocExamples_index_at(env):
    """Test index_at() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"arr": ["a", "b", "c", "d"]}')
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "index_at(arr, `1`)")
    r.assertEqual(result, '"b"')

    result = r.execute_command("JSON.JMESPATH", "doc", "index_at(arr, `-1`)")
    r.assertEqual(result, '"d"')


def testDocExamples_includes(env):
    """Test includes() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"arr": [1, 2, 3]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "includes(arr, `2`)")
    r.assertEqual(result, "true")


def testDocExamples_find_index(env):
    """Test find_index() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"arr": ["a", "b", "c"]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "find_index(arr, 'b')")
    r.assertEqual(result, "1")


def testDocExamples_first(env):
    """Test first() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"items": [1, 2, 3]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "first(items)")
    r.assertEqual(result, "1")


def testDocExamples_last(env):
    """Test last() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"items": [1, 2, 3]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "last(items)")
    r.assertEqual(result, "3")


def testDocExamples_difference(env):
    """Test difference() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"a": [1, 2, 3, 4], "b": [2, 4]}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "difference(a, b)")
    r.assertEqual(json.loads(result), [1, 3])


def testDocExamples_intersection(env):
    """Test intersection() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"a": [1, 2, 3], "b": [2, 3, 4]}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "intersection(a, b)")
    r.assertEqual(json.loads(result), [2, 3])


def testDocExamples_union(env):
    """Test union() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"a": [1, 2], "b": [2, 3]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "union(a, b)")
    r.assertEqual(json.loads(result), [1, 2, 3])


def testDocExamples_frequencies(env):
    """Test frequencies() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"tags": ["redis", "json", "redis", "nosql", "json", "redis"]}',
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "frequencies(tags)")
    parsed = json.loads(result)
    r.assertEqual(parsed["redis"], 3)
    r.assertEqual(parsed["json"], 2)
    r.assertEqual(parsed["nosql"], 1)


def testDocExamples_rotate(env):
    """Test rotate() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"arr": [1, 2, 3, 4, 5]}'))

    result = r.execute_command("JSON.JMESPATH", "doc", "rotate(arr, `2`)")
    r.assertEqual(json.loads(result), [3, 4, 5, 1, 2])

    result = r.execute_command("JSON.JMESPATH", "doc", "rotate(arr, `-1`)")
    r.assertEqual(json.loads(result), [5, 1, 2, 3, 4])


def testDocExamples_cartesian(env):
    """Test cartesian() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"a": [1, 2], "b": ["x", "y"]}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "cartesian(a, b)")
    r.assertEqual(json.loads(result), [[1, "x"], [1, "y"], [2, "x"], [2, "y"]])


# -----------------------------------------------------------------------------
# Object Functions
# -----------------------------------------------------------------------------


def testDocExamples_entries(env):
    """Test entries() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"a": 1, "b": 2}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "entries(@)")
    parsed = json.loads(result)
    r.assertEqual(len(parsed), 2)
    # Check structure
    r.assertTrue(all("key" in e and "value" in e for e in parsed))


def testDocExamples_from_entries(env):
    """Test from_entries() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET", "doc", "$", '[{"key":"a","value":1},{"key":"b","value":2}]'
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "from_entries(@)")
    r.assertEqual(json.loads(result), {"a": 1, "b": 2})


def testDocExamples_pick(env):
    """Test pick() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"name": "Alice", "age": 30, "email": "alice@example.com", "password": "secret"}',
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "pick(@, ['name', 'email'])")
    parsed = json.loads(result)
    r.assertEqual(parsed["name"], "Alice")
    r.assertEqual(parsed["email"], "alice@example.com")
    r.assertTrue("password" not in parsed)


def testDocExamples_omit(env):
    """Test omit() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET", "doc", "$", '{"name": "Alice", "age": 30, "password": "secret"}'
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "omit(@, ['password'])")
    parsed = json.loads(result)
    r.assertEqual(parsed["name"], "Alice")
    r.assertEqual(parsed["age"], 30)
    r.assertTrue("password" not in parsed)


def testDocExamples_deep_merge(env):
    """Test deep_merge() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"a": {"x": 1}, "b": {"y": 2}}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "deep_merge(a, b)")
    r.assertEqual(json.loads(result), {"x": 1, "y": 2})


def testDocExamples_invert(env):
    """Test invert() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"a": "1", "b": "2", "c": "3"}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "invert(@)")
    r.assertEqual(json.loads(result), {"1": "a", "2": "b", "3": "c"})


def testDocExamples_flatten_keys(env):
    """Test flatten_keys() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"user": {"name": "Alice", "address": {"city": "NYC", "zip": "10001"}}}',
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "flatten_keys(@)")
    parsed = json.loads(result)
    r.assertEqual(parsed["user.name"], "Alice")
    r.assertEqual(parsed["user.address.city"], "NYC")


def testDocExamples_unflatten_keys(env):
    """Test unflatten_keys() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET", "doc", "$", '{"user.name": "Alice", "user.address.city": "NYC"}'
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "unflatten_keys(@)")
    parsed = json.loads(result)
    r.assertEqual(parsed["user"]["name"], "Alice")
    r.assertEqual(parsed["user"]["address"]["city"], "NYC")


# -----------------------------------------------------------------------------
# Math Functions
# -----------------------------------------------------------------------------


def testDocExamples_round(env):
    """Test round() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"n": 3.14159}'))

    result = r.execute_command("JSON.JMESPATH", "doc", "round(n)")
    r.assertEqual(json.loads(result), 3.0)

    result = r.execute_command("JSON.JMESPATH", "doc", "round(n, `2`)")
    r.assertEqual(json.loads(result), 3.14)


def testDocExamples_floor_fn(env):
    """Test floor_fn() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"n": 3.7}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "floor_fn(n)")
    r.assertEqual(result, "3")


def testDocExamples_ceil_fn(env):
    """Test ceil_fn() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"n": 3.2}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "ceil_fn(n)")
    r.assertEqual(result, "4")


def testDocExamples_abs_fn(env):
    """Test abs_fn() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"n": -5}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "abs_fn(n)")
    r.assertEqual(json.loads(result), 5.0)


def testDocExamples_mod_fn(env):
    """Test mod_fn() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"a": 10, "b": 3}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "mod_fn(a, b)")
    r.assertEqual(json.loads(result), 1.0)


def testDocExamples_pow(env):
    """Test pow() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"base": 2, "exp": 3}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "pow(base, exp)")
    r.assertEqual(json.loads(result), 8.0)


def testDocExamples_sqrt(env):
    """Test sqrt() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"n": 16}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "sqrt(n)")
    r.assertEqual(json.loads(result), 4.0)


def testDocExamples_log(env):
    """Test log() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"n": 100}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "log(n, `10`)")
    r.assertEqual(json.loads(result), 2.0)


def testDocExamples_clamp(env):
    """Test clamp() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"n": 15}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "clamp(n, `0`, `10`)")
    r.assertEqual(json.loads(result), 10.0)


def testDocExamples_median(env):
    """Test median() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"scores": [1, 3, 5, 7, 9]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "median(scores)")
    r.assertEqual(json.loads(result), 5.0)


def testDocExamples_sign(env):
    """Test sign() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"pos": 42, "neg": -17, "zero": 0}')
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "sign(pos)")
    r.assertEqual(result, "1")

    result = r.execute_command("JSON.JMESPATH", "doc", "sign(neg)")
    r.assertEqual(result, "-1")

    result = r.execute_command("JSON.JMESPATH", "doc", "sign(zero)")
    r.assertEqual(result, "0")


# -----------------------------------------------------------------------------
# Type Functions
# -----------------------------------------------------------------------------


def testDocExamples_to_string(env):
    """Test to_string() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"n": 42}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "to_string(n)")
    r.assertEqual(result, '"42"')


def testDocExamples_to_number(env):
    """Test to_number() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "42"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "to_number(s)")
    r.assertEqual(json.loads(result), 42.0)


def testDocExamples_to_boolean(env):
    """Test to_boolean() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "hello"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "to_boolean(s)")
    r.assertEqual(result, "true")

    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": ""}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "to_boolean(s)")
    r.assertEqual(result, "false")


def testDocExamples_type_of(env):
    """Test type_of() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"a": [1,2,3]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "type_of(a)")
    r.assertEqual(result, '"array"')


def testDocExamples_is_string(env):
    """Test is_string() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "hello"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_string(s)")
    r.assertEqual(result, "true")


def testDocExamples_is_number(env):
    """Test is_number() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"n": 42}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_number(n)")
    r.assertEqual(result, "true")


def testDocExamples_is_array(env):
    """Test is_array() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"a": [1,2,3]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_array(a)")
    r.assertEqual(result, "true")


def testDocExamples_is_object(env):
    """Test is_object() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"o": {"key": "value"}}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_object(o)")
    r.assertEqual(result, "true")


def testDocExamples_is_null(env):
    """Test is_null() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"n": null}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_null(n)")
    r.assertEqual(result, "true")


# -----------------------------------------------------------------------------
# Hash Functions
# -----------------------------------------------------------------------------


def testDocExamples_md5(env):
    """Test md5() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"data": "hello world"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "md5(data)")
    r.assertEqual(result, '"5eb63bbbe01eeed093cb22bb8f5acdc3"')


def testDocExamples_sha1(env):
    """Test sha1() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"data": "hello world"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "sha1(data)")
    r.assertEqual(result, '"2aae6c35c94fcfb415dbe95f408b9ce91ee846ed"')


def testDocExamples_sha256(env):
    """Test sha256() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"data": "hello world"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "sha256(data)")
    r.assertEqual(
        result, '"b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"'
    )


def testDocExamples_crc32(env):
    """Test crc32() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"data": "hello world"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "crc32(data)")
    r.assertEqual(result, "222957957")


# -----------------------------------------------------------------------------
# Encoding Functions
# -----------------------------------------------------------------------------


def testDocExamples_base64_encode(env):
    """Test base64_encode() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"message": "Hello, World!"}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "base64_encode(message)")
    r.assertEqual(result, '"SGVsbG8sIFdvcmxkIQ=="')


def testDocExamples_base64_decode(env):
    """Test base64_decode() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"encoded": "SGVsbG8sIFdvcmxkIQ=="}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "base64_decode(encoded)")
    r.assertEqual(result, '"Hello, World!"')


def testDocExamples_hex_encode(env):
    """Test hex_encode() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"data": "hello"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "hex_encode(data)")
    r.assertEqual(result, '"68656c6c6f"')


def testDocExamples_hex_decode(env):
    """Test hex_decode() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"hex": "68656c6c6f"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "hex_decode(hex)")
    r.assertEqual(result, '"hello"')


def testDocExamples_json_encode(env):
    """Test json_encode() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"arr": [1, 2, 3]}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "json_encode(arr)")
    r.assertEqual(result, '"[1,2,3]"')


def testDocExamples_json_decode(env):
    """Test json_decode() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"json_str": "{\\"name\\": \\"Bob\\", \\"age\\": 30}"}',
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "json_decode(json_str)")
    parsed = json.loads(result)
    r.assertEqual(parsed["name"], "Bob")
    r.assertEqual(parsed["age"], 30)


# -----------------------------------------------------------------------------
# URL Functions
# -----------------------------------------------------------------------------


def testDocExamples_url_encode(env):
    """Test url_encode() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"q": "hello world"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "url_encode(q)")
    r.assertEqual(result, '"hello%20world"')


def testDocExamples_url_decode(env):
    """Test url_decode() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"q": "hello%20world"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "url_decode(q)")
    r.assertEqual(result, '"hello world"')


def testDocExamples_url_parse(env):
    """Test url_parse() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET", "doc", "$", '{"url": "https://example.com:8080/path?query=1"}'
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "url_parse(url).host")
    r.assertEqual(result, '"example.com"')


# -----------------------------------------------------------------------------
# Path Functions
# -----------------------------------------------------------------------------


def testDocExamples_path_basename(env):
    """Test path_basename() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"p": "/home/user/file.txt"}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "path_basename(p)")
    r.assertEqual(result, '"file.txt"')


def testDocExamples_path_dirname(env):
    """Test path_dirname() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"p": "/home/user/file.txt"}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "path_dirname(p)")
    r.assertEqual(result, '"/home/user"')


def testDocExamples_path_ext(env):
    """Test path_ext() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"p": "/home/user/file.txt"}')
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "path_ext(p)")
    r.assertEqual(result, '".txt"')


def testDocExamples_path_join(env):
    """Test path_join() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET", "doc", "$", '{"parts": ["home", "user", "docs", "file.txt"]}'
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "path_join(parts)")
    r.assertEqual(result, '"home/user/docs/file.txt"')


# -----------------------------------------------------------------------------
# Validation Functions
# -----------------------------------------------------------------------------


def testDocExamples_is_email(env):
    """Test is_email() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"e": "user@example.com"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_email(e)")
    r.assertEqual(result, "true")

    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"e": "not-an-email"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_email(e)")
    r.assertEqual(result, "false")


def testDocExamples_is_url(env):
    """Test is_url() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET", "doc", "$", '{"u": "https://example.com/path?query=1"}'
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "is_url(u)")
    r.assertEqual(result, "true")


def testDocExamples_is_uuid(env):
    """Test is_uuid() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET", "doc", "$", '{"id": "550e8400-e29b-41d4-a716-446655440000"}'
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "is_uuid(id)")
    r.assertEqual(result, "true")


def testDocExamples_is_ipv4(env):
    """Test is_ipv4() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"ip": "192.168.1.1"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_ipv4(ip)")
    r.assertEqual(result, "true")

    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"ip": "256.1.1.1"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_ipv4(ip)")
    r.assertEqual(result, "false")


def testDocExamples_is_ipv6(env):
    """Test is_ipv6() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"ip": "::1"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_ipv6(ip)")
    r.assertEqual(result, "true")


def testDocExamples_is_empty(env):
    """Test is_empty() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"s": "", "arr": [], "obj": {}}')
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "is_empty(s)")
    r.assertEqual(result, "true")

    result = r.execute_command("JSON.JMESPATH", "doc", "is_empty(arr)")
    r.assertEqual(result, "true")

    result = r.execute_command("JSON.JMESPATH", "doc", "is_empty(obj)")
    r.assertEqual(result, "true")


def testDocExamples_is_blank(env):
    """Test is_blank() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "   "}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_blank(s)")
    r.assertEqual(result, "true")

    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "  hello  "}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_blank(s)")
    r.assertEqual(result, "false")


def testDocExamples_is_json(env):
    """Test is_json() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "{\\"a\\": 1}"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_json(s)")
    r.assertEqual(result, "true")

    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"s": "{not json}"}'))
    result = r.execute_command("JSON.JMESPATH", "doc", "is_json(s)")
    r.assertEqual(result, "false")


# -----------------------------------------------------------------------------
# Utility Functions
# -----------------------------------------------------------------------------


def testDocExamples_default(env):
    """Test default() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"name": null, "role": "admin"}')
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "default(name, 'Unknown')")
    r.assertEqual(result, '"Unknown"')

    result = r.execute_command("JSON.JMESPATH", "doc", "default(role, 'guest')")
    r.assertEqual(result, '"admin"')


def testDocExamples_if(env):
    """Test if() examples from docs"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"age": 25}'))
    result = r.execute_command(
        "JSON.JMESPATH", "doc", "if(age >= `18`, 'adult', 'minor')"
    )
    r.assertEqual(result, '"adult"')


def testDocExamples_coalesce(env):
    """Test coalesce() examples from docs"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET", "doc", "$", '{"a": null, "b": null, "c": "found"}'
        )
    )
    result = r.execute_command("JSON.JMESPATH", "doc", "coalesce(a, b, c)")
    r.assertEqual(result, '"found"')
