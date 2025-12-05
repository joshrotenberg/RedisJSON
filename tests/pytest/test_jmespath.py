# -*- coding: utf-8 -*-

"""
Tests for JSON.JMESPATH command.

JMESPath is a query language for JSON that supports projections, filters,
pipe expressions, and functions. Unlike JSONPath, JMESPath is read-only.
"""

import json

from RLTest import Defaults, Env

Defaults.decode_responses = True

# -----------------------------------------------------------------------------
# Basic Field Access
# -----------------------------------------------------------------------------


def testJmespathBasicFieldAccess(env):
    """Test basic field access with JMESPath"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"name": "Alice", "age": 30}')
    )

    # Access a string field
    result = r.execute_command("JSON.JMESPATH", "doc", "name")
    r.assertEqual(result, '"Alice"')

    # Access a number field
    result = r.execute_command("JSON.JMESPATH", "doc", "age")
    r.assertEqual(result, "30")


def testJmespathNestedFieldAccess(env):
    """Test nested field access with JMESPath"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"user": {"name": "Alice", "profile": {"city": "NYC"}}}',
        )
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "user.name")
    r.assertEqual(result, '"Alice"')

    result = r.execute_command("JSON.JMESPATH", "doc", "user.profile.city")
    r.assertEqual(result, '"NYC"')


def testJmespathArrayIndex(env):
    """Test array indexing with JMESPath"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"items": ["a", "b", "c", "d"]}')
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "items[0]")
    r.assertEqual(result, '"a"')

    result = r.execute_command("JSON.JMESPATH", "doc", "items[2]")
    r.assertEqual(result, '"c"')

    # Negative indexing
    result = r.execute_command("JSON.JMESPATH", "doc", "items[-1]")
    r.assertEqual(result, '"d"')


# -----------------------------------------------------------------------------
# Projections
# -----------------------------------------------------------------------------


def testJmespathProjection(env):
    """Test array projection with JMESPath"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"people": [{"name": "Alice"}, {"name": "Bob"}, {"name": "Carol"}]}',
        )
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "people[*].name")
    r.assertEqual(json.loads(result), ["Alice", "Bob", "Carol"])


def testJmespathSlicing(env):
    """Test array slicing with JMESPath"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"items": [0, 1, 2, 3, 4, 5]}')
    )

    # First 3 items
    result = r.execute_command("JSON.JMESPATH", "doc", "items[:3]")
    r.assertEqual(json.loads(result), [0, 1, 2])

    # Last 2 items
    result = r.execute_command("JSON.JMESPATH", "doc", "items[-2:]")
    r.assertEqual(json.loads(result), [4, 5])


# -----------------------------------------------------------------------------
# Filters
# -----------------------------------------------------------------------------


def testJmespathFilter(env):
    """Test filter expressions with JMESPath"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"items": [{"price": 10}, {"price": 25}, {"price": 5}, {"price": 30}]}',
        )
    )

    # Filter items with price > 10
    result = r.execute_command("JSON.JMESPATH", "doc", "items[?price > `10`].price")
    r.assertEqual(json.loads(result), [25, 30])

    # Filter with equality
    result = r.execute_command("JSON.JMESPATH", "doc", "items[?price == `25`]")
    r.assertEqual(json.loads(result), [{"price": 25}])


def testJmespathFilterWithStrings(env):
    """Test filter expressions with string comparisons"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"users": [{"name": "Alice", "role": "admin"}, {"name": "Bob", "role": "user"}, {"name": "Carol", "role": "admin"}]}',
        )
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "users[?role == 'admin'].name")
    r.assertEqual(json.loads(result), ["Alice", "Carol"])


def testJmespathFilterWithBoolean(env):
    """Test filter expressions with boolean values"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"items": [{"name": "A", "active": true}, {"name": "B", "active": false}, {"name": "C", "active": true}]}',
        )
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "items[?active].name")
    r.assertEqual(json.loads(result), ["A", "C"])


def testJmespathFilterWithLogicalOperators(env):
    """Test filter expressions with AND/OR"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"products": [{"name": "A", "price": 100, "in_stock": true}, {"name": "B", "price": 50, "in_stock": false}, {"name": "C", "price": 75, "in_stock": true}]}',
        )
    )

    # AND condition
    result = r.execute_command(
        "JSON.JMESPATH", "doc", "products[?price > `60` && in_stock].name"
    )
    r.assertEqual(json.loads(result), ["A", "C"])

    # OR condition
    result = r.execute_command(
        "JSON.JMESPATH", "doc", "products[?price > `90` || !in_stock].name"
    )
    r.assertEqual(json.loads(result), ["A", "B"])


# -----------------------------------------------------------------------------
# Pipe Expressions
# -----------------------------------------------------------------------------


def testJmespathPipe(env):
    """Test pipe expressions with JMESPath"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET", "doc", "$", '{"names": ["charlie", "alice", "bob"]}'
        )
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "names | sort(@)")
    r.assertEqual(json.loads(result), ["alice", "bob", "charlie"])

    # Pipe with reverse
    result = r.execute_command("JSON.JMESPATH", "doc", "names | sort(@) | reverse(@)")
    r.assertEqual(json.loads(result), ["charlie", "bob", "alice"])


# -----------------------------------------------------------------------------
# Built-in Functions
# -----------------------------------------------------------------------------


def testJmespathLengthFunction(env):
    """Test length() function"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"items": [1, 2, 3, 4, 5]}'))

    result = r.execute_command("JSON.JMESPATH", "doc", "length(items)")
    r.assertEqual(result, "5")


def testJmespathKeysValuesFunction(env):
    """Test keys() and values() functions"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"a": 1, "b": 2, "c": 3}'))

    result = r.execute_command("JSON.JMESPATH", "doc", "keys(@)")
    keys = json.loads(result)
    r.assertEqual(sorted(keys), ["a", "b", "c"])

    result = r.execute_command("JSON.JMESPATH", "doc", "values(@)")
    values = json.loads(result)
    r.assertEqual(sorted(values), [1, 2, 3])


def testJmespathMinMaxFunctions(env):
    """Test min() and max() functions"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"numbers": [5, 2, 8, 1, 9]}')
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "min(numbers)")
    r.assertEqual(result, "1")

    result = r.execute_command("JSON.JMESPATH", "doc", "max(numbers)")
    r.assertEqual(result, "9")


def testJmespathSumAvgFunctions(env):
    """Test sum() and avg() functions"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"numbers": [10, 20, 30, 40]}')
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "sum(numbers)")
    r.assertEqual(json.loads(result), 100)

    result = r.execute_command("JSON.JMESPATH", "doc", "avg(numbers)")
    r.assertEqual(json.loads(result), 25.0)


def testJmespathContainsFunction(env):
    """Test contains() function"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"items": ["a", "b", "c"]}'))

    result = r.execute_command("JSON.JMESPATH", "doc", "contains(items, 'b')")
    r.assertEqual(result, "true")

    result = r.execute_command("JSON.JMESPATH", "doc", "contains(items, 'z')")
    r.assertEqual(result, "false")


def testJmespathSortByFunction(env):
    """Test sort_by() function"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"users": [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}, {"name": "Carol", "age": 35}]}',
        )
    )

    result = r.execute_command(
        "JSON.JMESPATH", "doc", "users | sort_by(@, &age) | [*].name"
    )
    r.assertEqual(json.loads(result), ["Bob", "Alice", "Carol"])


def testJmespathMaxByMinByFunctions(env):
    """Test max_by() and min_by() functions"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"users": [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}, {"name": "Carol", "age": 35}]}',
        )
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "max_by(users, &age).name")
    r.assertEqual(result, '"Carol"')

    result = r.execute_command("JSON.JMESPATH", "doc", "min_by(users, &age).name")
    r.assertEqual(result, '"Bob"')


# -----------------------------------------------------------------------------
# Multi-select
# -----------------------------------------------------------------------------


def testJmespathMultiSelectHash(env):
    """Test multi-select hash for reshaping data"""
    r = env
    r.assertOk(
        r.execute_command(
            "JSON.SET",
            "doc",
            "$",
            '{"person": {"firstName": "Alice", "lastName": "Smith", "age": 30}}',
        )
    )

    result = r.execute_command(
        "JSON.JMESPATH", "doc", "person.{name: firstName, years: age}"
    )
    parsed = json.loads(result)
    r.assertEqual(parsed["name"], "Alice")
    r.assertEqual(parsed["years"], 30)


def testJmespathMultiSelectList(env):
    """Test multi-select list"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"a": 1, "b": 2, "c": 3}'))

    result = r.execute_command("JSON.JMESPATH", "doc", "[a, b, c]")
    r.assertEqual(json.loads(result), [1, 2, 3])


# -----------------------------------------------------------------------------
# Flatten
# -----------------------------------------------------------------------------


def testJmespathFlatten(env):
    """Test array flattening"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"arrays": [[1, 2], [3, 4], [5]]}')
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "arrays[]")
    r.assertEqual(json.loads(result), [1, 2, 3, 4, 5])


# -----------------------------------------------------------------------------
# Missing Key and Null Handling
# -----------------------------------------------------------------------------


def testJmespathMissingKey(env):
    """Test that missing Redis key returns null"""
    r = env
    result = r.execute_command("JSON.JMESPATH", "nonexistent", "foo")
    r.assertIsNone(result)


def testJmespathMissingField(env):
    """Test that missing field in document returns JSON null"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", '{"foo": "bar"}'))

    result = r.execute_command("JSON.JMESPATH", "doc", "missing")
    r.assertEqual(result, "null")


def testJmespathEmptyFilterResult(env):
    """Test that filter with no matches returns empty array"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"items": [{"x": 1}, {"x": 2}]}')
    )

    result = r.execute_command("JSON.JMESPATH", "doc", "items[?x > `100`]")
    r.assertEqual(result, "[]")


# -----------------------------------------------------------------------------
# Error Handling
# -----------------------------------------------------------------------------


def testJmespathInvalidExpression(env):
    """Test that invalid expression returns error"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", "{}"))

    r.expect("JSON.JMESPATH", "doc", "[invalid").raiseError()


def testJmespathWrongArity(env):
    """Test wrong number of arguments"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", "{}"))

    # Missing expression
    r.expect("JSON.JMESPATH", "doc").raiseError()

    # Missing key
    r.expect("JSON.JMESPATH").raiseError()


def testJmespathUnknownArgument(env):
    """Test unknown optional argument"""
    r = env
    r.assertOk(r.execute_command("JSON.SET", "doc", "$", "{}"))

    r.expect("JSON.JMESPATH", "doc", "@", "UNKNOWN", "arg").raiseError()


# -----------------------------------------------------------------------------
# Formatting Options
# -----------------------------------------------------------------------------


def testJmespathFormattingOptions(env):
    """Test INDENT, NEWLINE, SPACE formatting options"""
    r = env
    r.assertOk(
        r.execute_command("JSON.SET", "doc", "$", '{"name": "Alice", "age": 30}')
    )

    # With formatting
    result = r.execute_command(
        "JSON.JMESPATH", "doc", "@", "INDENT", "  ", "NEWLINE", "\n", "SPACE", " "
    )
    r.assertTrue("\n" in result)
    r.assertTrue("  " in result)


# -----------------------------------------------------------------------------
# Complex Real-World Examples
# -----------------------------------------------------------------------------


def testJmespathEcommerceExample(env):
    """Test e-commerce product analysis example"""
    r = env
    products = json.dumps(
        [
            {
                "name": "Laptop",
                "price": 999,
                "category": "electronics",
                "rating": 4.5,
                "in_stock": True,
            },
            {
                "name": "Headphones",
                "price": 199,
                "category": "electronics",
                "rating": 4.8,
                "in_stock": False,
            },
            {
                "name": "Desk Chair",
                "price": 299,
                "category": "furniture",
                "rating": 4.2,
                "in_stock": True,
            },
            {
                "name": "Monitor",
                "price": 399,
                "category": "electronics",
                "rating": 4.6,
                "in_stock": True,
            },
        ]
    )
    r.assertOk(r.execute_command("JSON.SET", "products", "$", products))

    # Get all categories sorted
    result = r.execute_command("JSON.JMESPATH", "products", "[*].category | sort(@)")
    r.assertEqual(
        json.loads(result), ["electronics", "electronics", "electronics", "furniture"]
    )

    # Get in-stock electronics sorted by rating
    result = r.execute_command(
        "JSON.JMESPATH",
        "products",
        "[?in_stock && category == 'electronics'] | sort_by(@, &rating) | reverse(@) | [*].name",
    )
    r.assertEqual(json.loads(result), ["Monitor", "Laptop"])


def testJmespathApiResponseTransform(env):
    """Test API response transformation example"""
    r = env
    response = json.dumps(
        {
            "status": "success",
            "data": {
                "orders": [
                    {
                        "id": "A001",
                        "items": [{"sku": "X1", "qty": 2}, {"sku": "X2", "qty": 1}],
                        "total": 150.00,
                    },
                    {
                        "id": "A002",
                        "items": [{"sku": "X1", "qty": 1}, {"sku": "X3", "qty": 3}],
                        "total": 200.00,
                    },
                    {"id": "A003", "items": [{"sku": "X2", "qty": 5}], "total": 75.00},
                ]
            },
        }
    )
    r.assertOk(r.execute_command("JSON.SET", "api_response", "$", response))

    # Get all SKUs across all orders (flattened and sorted)
    result = r.execute_command(
        "JSON.JMESPATH",
        "api_response",
        "data.orders[*].items[].sku | sort(@)",
    )
    r.assertEqual(json.loads(result), ["X1", "X1", "X2", "X2", "X3"])

    # Get order with highest total
    result = r.execute_command(
        "JSON.JMESPATH",
        "api_response",
        "data.orders | max_by(@, &total) | {id: id, total: total}",
    )
    parsed = json.loads(result)
    r.assertEqual(parsed["id"], "A002")
    r.assertEqual(parsed["total"], 200.0)


def testJmespathConfigManagement(env):
    """Test configuration management example"""
    r = env
    config = json.dumps(
        {
            "services": [
                {
                    "name": "api",
                    "instances": [
                        {"host": "10.0.1.1", "port": 8080, "healthy": True},
                        {"host": "10.0.1.2", "port": 8080, "healthy": False},
                    ],
                },
                {
                    "name": "web",
                    "instances": [{"host": "10.0.2.1", "port": 3000, "healthy": True}],
                },
                {
                    "name": "worker",
                    "instances": [
                        {"host": "10.0.3.1", "port": 9000, "healthy": True},
                        {"host": "10.0.3.2", "port": 9000, "healthy": True},
                    ],
                },
            ]
        }
    )
    r.assertOk(r.execute_command("JSON.SET", "config", "$", config))

    # Find services with any unhealthy instances
    result = r.execute_command(
        "JSON.JMESPATH", "config", "services[?length(instances[?!healthy]) > `0`].name"
    )
    r.assertEqual(json.loads(result), ["api"])

    # Count healthy vs total instances per service
    result = r.execute_command(
        "JSON.JMESPATH",
        "config",
        "services[*].{service: name, healthy: length(instances[?healthy]), total: length(instances)}",
    )
    parsed = json.loads(result)
    r.assertEqual(len(parsed), 3)
    # Find the api service entry
    api_entry = next(s for s in parsed if s["service"] == "api")
    r.assertEqual(api_entry["healthy"], 1)
    r.assertEqual(api_entry["total"], 2)
