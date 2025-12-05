# JMESPath Support in RedisJSON

> **TL;DR:** `JSON.JMESPATH` provides JMESPath query support for RedisJSON - a powerful read-only query language with 26 standard + 65 custom functions for data extraction and transformation. Compiled expressions are cached for performance.

RedisJSON extends its query capabilities with [JMESPath](https://jmespath.org/), a powerful query language for JSON. This document covers the `JSON.JMESPATH` command and the custom functions available in this implementation.

## Why JMESPath?

**One command instead of multiple round trips + client-side processing.**

Without JMESPath (multiple operations):
```python
# Fetch data
data = redis.execute_command('JSON.GET', 'users', '$[?(@.active)]')
users = json.loads(data)

# Client-side processing
result = [{"name": u["name"].upper(), "age": u["age"]} for u in users]
result.sort(key=lambda x: x["age"])
avg_age = sum(u["age"] for u in result) / len(result)
```

With JMESPath (single command):
```bash
JSON.JMESPATH users "[?active] | sort_by(@, &age) | {users: [*].{name: upper(name), age: age}, avg_age: avg([*].age)}"
```

**What this means:**
- Fewer network round trips
- No client-side JSON parsing/processing
- Less code to maintain
- Processing happens at the data, not after fetching it

**Capabilities JSONPath doesn't have:**

| Capability | Example |
|------------|---------|
| Reshape output | `{id: id, username: name}` |
| Aggregations | `sum([*].price)`, `avg([*].age)` |
| Pipe operations | `filter \| sort \| [:10]` |
| String transforms | `upper(name)`, `split(tags, ',')` |
| Computed fields | `{adult: age >= \`18\`}` |
| Null handling | `default(nickname, name)` |
| Set operations | `unique([*].category)` |

## When to Use JMESPath vs JSONPath

Both query languages are available in RedisJSON. Choose based on your needs:

| Use Case | Recommendation |
|----------|----------------|
| **Complex transformations** | JMESPath - pipes, multiselect, rich functions |
| **Data reshaping** | JMESPath - `{newKey: oldKey}` syntax |
| **Aggregations** | JMESPath - `sum()`, `avg()`, `max()`, etc. |
| **Mutations (SET, DEL, etc.)** | JSONPath - JMESPath is read-only |
| **Simple field extraction** | Either works fine |
| **Existing JSONPath tooling** | JSONPath - for compatibility |
| **Cross-language portability** | JMESPath - same syntax in Python, JS, Go, etc. |

**Rule of thumb:** Use JMESPath for complex read queries, JSONPath for writes and simple reads.

## Migrating from JSONPath

Common JSONPath patterns and their JMESPath equivalents:

| Task | JSONPath | JMESPath |
|------|----------|----------|
| Get field | `$.name` | `name` |
| Array index | `$[0]` | `[0]` |
| All elements | `$[*]` | `[*]` |
| Nested field | `$.user.name` | `user.name` |
| Filter | `$[?(@.age > 25)]` | `[?age > \`25\`]` |
| Filter + field | `$[?(@.active)].name` | `[?active].name` |
| Multiple fields | N/A (multiple queries) | `{name: name, age: age}` |
| Wildcard descent | `$..name` | `[*].name` (one level) |

**Key differences:**
- JMESPath uses backticks for literals: `` `25` `` not `25`
- JMESPath filter syntax: `[?age > \`25\`]` not `[?(@.age > 25)]`
- JMESPath has no recursive descent (`..`), use explicit paths
- JMESPath can reshape in one query with multiselect `{}`

```bash
# JSONPath: Get names of users over 25
redis> JSON.GET users '$[?(@.age > 25)].name'

# JMESPath equivalent:
redis> JSON.JMESPATH users "[?age > `25`].name"

# JMESPath bonus - reshape the output:
redis> JSON.JMESPATH users "[?age > `25`].{username: name, years: age}"
```

## Real-World Use Cases

### E-commerce: Order Processing
```bash
# Get pending orders with totals > $100
redis> JSON.JMESPATH orders "[?status == 'pending' && total > `100`].{
  order_id: id,
  customer: customer.email,
  amount: total
}"

# Calculate order statistics
redis> JSON.JMESPATH orders "{
  total_orders: length(@),
  total_revenue: sum([*].total),
  avg_order: avg([*].total),
  pending_count: length([?status == 'pending'])
}"
```

### User Management: Session Data
```bash
# Find active sessions for a user role
redis> JSON.JMESPATH sessions "[?user.role == 'admin' && expires > now()].{
  user: user.email,
  ip: ip_address,
  expires_in: expires
}"

# Get unique roles across all sessions
redis> JSON.JMESPATH sessions "unique([*].user.role)"
```

### IoT: Sensor Data
```bash
# Get sensors with readings above threshold
redis> JSON.JMESPATH sensors "[?last_reading.value > `80`].{
  sensor_id: id,
  location: location,
  value: last_reading.value,
  alert: last_reading.value > `90`
}"

# Aggregate sensor statistics by location
redis> JSON.JMESPATH sensors "{
  total_sensors: length(@),
  avg_reading: avg([*].last_reading.value),
  max_reading: max([*].last_reading.value),
  locations: unique([*].location)
}"
```

### API Response Transformation
```bash
# Transform nested API response to flat structure
redis> JSON.JMESPATH api_response "data.users[*].{
  id: id,
  full_name: concat([first_name, last_name], ' '),
  email: lower(email),
  role: default(role, 'user'),
  active: is_active
}"

# Extract pagination info alongside data
redis> JSON.JMESPATH api_response "{
  users: data.users[*].name,
  page: meta.current_page,
  total: meta.total_count,
  has_more: meta.current_page < meta.total_pages
}"
```

### Log Analysis
```bash
# Find error logs from the last hour
redis> JSON.JMESPATH logs "[?level == 'error' && timestamp > `1699900000`].{
  time: timestamp,
  message: message,
  source: source,
  trace: stack_trace
} | sort_by(@, &time) | reverse(@) | [:10]"

# Count logs by level
redis> JSON.JMESPATH logs "{
  errors: length([?level == 'error']),
  warnings: length([?level == 'warning']),
  info: length([?level == 'info'])
}"
```

### Configuration Management
```bash
# Merge defaults with overrides
redis> JSON.JMESPATH config "merge(defaults, overrides)"

# Get feature flags for a specific environment
redis> JSON.JMESPATH features "[?environments[?@ == 'production']].{
  name: name,
  enabled: enabled,
  rollout: rollout_percentage
}"
```

## Command Syntax

```
JSON.JMESPATH key expression
    [INDENT indent-string]
    [NEWLINE newline-string]
    [SPACE space-string]
    [FORMAT {STRING|EXPAND}]
```

### Parameters

| Parameter | Description |
|-----------|-------------|
| `key` | The Redis key containing the JSON document |
| `expression` | A JMESPath expression to evaluate |
| `INDENT` | String to use for indentation (pretty printing) |
| `NEWLINE` | String to use for line breaks |
| `SPACE` | String to use after colons |
| `FORMAT` | Output format: `STRING` (JSON string) or `EXPAND` (RESP3 native types) |

### Return Value

- **RESP2**: JSON string containing the query result
- **RESP3 with FORMAT STRING**: JSON string (default)
- **RESP3 with FORMAT EXPAND**: Native RESP3 types (arrays, maps, integers, etc.)
- **Null**: If the key does not exist

## Basic Examples

```bash
# Set up test data
redis> JSON.SET users $ '[{"name":"Alice","age":30,"role":"admin"},{"name":"Bob","age":25,"role":"user"}]'
OK

# Get all names
redis> JSON.JMESPATH users "[*].name"
"[\"Alice\",\"Bob\"]"

# Filter by age
redis> JSON.JMESPATH users "[?age > `25`].name"
"[\"Alice\"]"

# Get first user's info
redis> JSON.JMESPATH users "[0].{name: name, role: role}"
"{\"name\":\"Alice\",\"role\":\"admin\"}"

# Sort by age and get youngest
redis> JSON.JMESPATH users "sort_by(@, &age) | [0].name"
"\"Bob\""
```

## JMESPath vs JSONPath

| Feature | JMESPath | JSONPath |
|---------|----------|----------|
| **Purpose** | Query & Transform | Query & Mutate |
| **Projections** | `[*].field` | `$[*].field` |
| **Filters** | `[?expr]` | `$[?(@.expr)]` |
| **Pipes** | `expr1 \| expr2` | Not supported |
| **Multiselect** | `{a: f1, b: f2}` | Not supported |
| **Functions** | 26 built-in + 50 custom | Limited |
| **Mutations** | Read-only | Read/Write |

## Standard JMESPath Functions (26)

These functions are part of the official JMESPath specification and work identically in all implementations.

### Math Functions

| Function | Description | Example |
|----------|-------------|---------|
| `abs(n)` | Absolute value | `abs(temperature)` |
| `avg(arr)` | Average of array | `avg(scores)` |
| `ceil(n)` | Round up | `ceil(price)` |
| `floor(n)` | Round down | `floor(price)` |
| `max(arr)` | Maximum value | `max(prices)` |
| `min(arr)` | Minimum value | `min(prices)` |
| `sum(arr)` | Sum of array | `sum(quantities)` |

### String Functions

| Function | Description | Example |
|----------|-------------|---------|
| `contains(s, sub)` | Check substring | `contains(name, 'test')` |
| `ends_with(s, suffix)` | Check suffix | `ends_with(file, '.json')` |
| `starts_with(s, prefix)` | Check prefix | `starts_with(id, 'usr_')` |
| `join(sep, arr)` | Join array | `join(', ', names)` |
| `length(x)` | Length of string/array/object | `length(items)` |

### Array/Object Functions

| Function | Description | Example |
|----------|-------------|---------|
| `keys(obj)` | Get object keys | `keys(config)` |
| `values(obj)` | Get object values | `values(config)` |
| `merge(o1, o2)` | Merge objects | `merge(defaults, overrides)` |
| `reverse(arr)` | Reverse array | `reverse(history)` |
| `sort(arr)` | Sort array | `sort(names)` |
| `sort_by(arr, &expr)` | Sort by expression | `sort_by(users, &age)` |
| `max_by(arr, &expr)` | Max by expression | `max_by(items, &price)` |
| `min_by(arr, &expr)` | Min by expression | `min_by(items, &price)` |
| `map(&expr, arr)` | Transform elements | `map(&name, users)` |

### Type Functions

| Function | Description | Example |
|----------|-------------|---------|
| `to_string(x)` | Convert to string | `to_string(count)` |
| `to_number(s)` | Parse number | `to_number(quantity)` |
| `to_array(x)` | Wrap in array | `to_array(item)` |
| `type(x)` | Get type name | `type(value)` |
| `not_null(...)` | First non-null | `not_null(a, b, c)` |

---

## Custom Redis Functions (50)

These functions extend JMESPath with capabilities specific to RedisJSON. **Note:** Queries using these functions are not portable to other JMESPath implementations.

### String Functions (15)

#### `lower(string) -> string`
Convert string to lowercase.

```bash
redis> JSON.SET doc $ '{"name": "ALICE"}'
redis> JSON.JMESPATH doc "lower(name)"
"\"alice\""

# Case-insensitive filtering
redis> JSON.SET users $ '[{"name":"ALICE"},{"name":"Bob"}]'
redis> JSON.JMESPATH users "[?lower(name) == 'alice'].name"
"[\"ALICE\"]"
```

#### `upper(string) -> string`
Convert string to uppercase.

```bash
redis> JSON.JMESPATH doc "upper(name)"
"\"ALICE\""
```

#### `trim(string) -> string`
Remove leading and trailing whitespace.

```bash
redis> JSON.SET doc $ '{"input": "  hello world  "}'
redis> JSON.JMESPATH doc "trim(input)"
"\"hello world\""
```

#### `capitalize(string) -> string`
Capitalize first letter of string.

```bash
redis> JSON.SET doc $ '{"name": "alice"}'
redis> JSON.JMESPATH doc "capitalize(name)"
"\"Alice\""
```

#### `title(string) -> string`
Capitalize first letter of each word.

```bash
redis> JSON.SET doc $ '{"title": "hello world"}'
redis> JSON.JMESPATH doc "title(title)"
"\"Hello World\""
```

#### `split(string, delimiter) -> array`
Split string into array by delimiter.

```bash
redis> JSON.SET doc $ '{"tags": "redis,json,database"}'
redis> JSON.JMESPATH doc "split(tags, ',')"
"[\"redis\",\"json\",\"database\"]"
```

#### `replace(string, old, new) -> string`
Replace all occurrences of a substring.

```bash
redis> JSON.SET doc $ '{"text": "hello world"}'
redis> JSON.JMESPATH doc "replace(text, 'world', 'redis')"
"\"hello redis\""
```

#### `repeat(string, count) -> string`
Repeat a string n times.

```bash
redis> JSON.SET doc $ '{"s": "ab"}'
redis> JSON.JMESPATH doc "repeat(s, `3`)"
"\"ababab\""
```

#### `pad_left(string, width, char) -> string`
Pad string on the left to reach specified width.

```bash
redis> JSON.SET doc $ '{"id": "42"}'
redis> JSON.JMESPATH doc "pad_left(id, `5`, '0')"
"\"00042\""
```

#### `pad_right(string, width, char) -> string`
Pad string on the right to reach specified width.

```bash
redis> JSON.JMESPATH doc "pad_right(id, `5`, '-')"
"\"42---\""
```

#### `substr(string, start, length?) -> string`
Extract a substring. Negative start counts from end.

```bash
redis> JSON.SET doc $ '{"s": "hello world"}'
redis> JSON.JMESPATH doc "substr(s, `0`, `5`)"
"\"hello\""

redis> JSON.JMESPATH doc "substr(s, `-5`)"
"\"world\""
```

#### `slice(string, start, end?) -> string`
Extract substring by start/end indices. Supports negative indices.

```bash
redis> JSON.SET doc $ '{"s": "hello world"}'
redis> JSON.JMESPATH doc "slice(s, `0`, `5`)"
"\"hello\""

redis> JSON.JMESPATH doc "slice(s, `-5`)"
"\"world\""
```

#### `index_of(string, search) -> number`
Find first occurrence of substring. Returns -1 if not found.

```bash
redis> JSON.SET doc $ '{"s": "hello world"}'
redis> JSON.JMESPATH doc "index_of(s, 'world')"
"6"
```

#### `last_index_of(string, search) -> number`
Find last occurrence of substring. Returns -1 if not found.

```bash
redis> JSON.SET doc $ '{"s": "hello hello"}'
redis> JSON.JMESPATH doc "last_index_of(s, 'hello')"
"6"
```

#### `concat(array, separator?) -> string`
Join array of strings with optional separator.

```bash
redis> JSON.SET doc $ '{"parts": ["a", "b", "c"]}'
redis> JSON.JMESPATH doc "concat(parts, '-')"
"\"a-b-c\""
```

### Array Functions (17)

#### `unique(array) -> array`
Remove duplicate values, preserving order.

```bash
redis> JSON.SET doc $ '{"tags": ["a", "b", "a", "c", "b"]}'
redis> JSON.JMESPATH doc "unique(tags)"
"[\"a\",\"b\",\"c\"]"
```

#### `zip(array1, array2) -> array`
Pair corresponding elements from two arrays.

```bash
redis> JSON.SET doc $ '{"keys": ["a","b","c"], "vals": [1,2,3]}'
redis> JSON.JMESPATH doc "zip(keys, vals)"
"[[\"a\",1],[\"b\",2],[\"c\",3]]"
```

#### `chunk(array, size) -> array`
Split array into chunks of specified size.

```bash
redis> JSON.SET doc $ '{"items": [1,2,3,4,5,6,7]}'
redis> JSON.JMESPATH doc "chunk(items, `3`)"
"[[1,2,3],[4,5,6],[7]]"
```

#### `take(array, n) -> array`
Get first n elements of array.

```bash
redis> JSON.SET doc $ '{"arr": [1, 2, 3, 4, 5]}'
redis> JSON.JMESPATH doc "take(arr, `3`)"
"[1,2,3]"
```

#### `drop(array, n) -> array`
Skip first n elements of array.

```bash
redis> JSON.SET doc $ '{"arr": [1, 2, 3, 4, 5]}'
redis> JSON.JMESPATH doc "drop(arr, `2`)"
"[3,4,5]"
```

#### `flatten_deep(array) -> array`
Recursively flatten nested arrays.

```bash
redis> JSON.SET doc $ '{"arr": [1, [2, [3, [4]]]]}'
redis> JSON.JMESPATH doc "flatten_deep(arr)"
"[1,2,3,4]"
```

#### `compact(array) -> array`
Remove null and false values from array.

```bash
redis> JSON.SET doc $ '{"arr": [1, null, 2, false, 3]}'
redis> JSON.JMESPATH doc "compact(arr)"
"[1,2,3]"
```

#### `range(start, end, step?) -> array`
Generate array of numbers. Step defaults to 1.

```bash
redis> JSON.SET doc $ '{}'
redis> JSON.JMESPATH doc "range(`0`, `5`)"
"[0,1,2,3,4]"

redis> JSON.JMESPATH doc "range(`0`, `10`, `2`)"
"[0,2,4,6,8]"

redis> JSON.JMESPATH doc "range(`5`, `0`, `-1`)"
"[5,4,3,2,1]"
```

#### `index_at(array, index) -> element`
Get element at index. Supports negative indices.

```bash
redis> JSON.SET doc $ '{"arr": ["a", "b", "c", "d"]}'
redis> JSON.JMESPATH doc "index_at(arr, `1`)"
"\"b\""

redis> JSON.JMESPATH doc "index_at(arr, `-1`)"
"\"d\""
```

#### `includes(array, value) -> boolean`
Check if array contains value.

```bash
redis> JSON.SET doc $ '{"arr": [1, 2, 3]}'
redis> JSON.JMESPATH doc "includes(arr, `2`)"
"true"
```

#### `find_index(array, value) -> number`
Find index of value in array. Returns -1 if not found.

```bash
redis> JSON.SET doc $ '{"arr": ["a", "b", "c"]}'
redis> JSON.JMESPATH doc "find_index(arr, 'b')"
"1"
```

#### `first(array) -> any`
Get first element of array, or null if empty.

```bash
redis> JSON.SET doc $ '{"items": [1, 2, 3]}'
redis> JSON.JMESPATH doc "first(items)"
"1"

redis> JSON.JMESPATH doc "first([])"  # Empty array
"null"
```

#### `last(array) -> any`
Get last element of array, or null if empty.

```bash
redis> JSON.SET doc $ '{"items": [1, 2, 3]}'
redis> JSON.JMESPATH doc "last(items)"
"3"
```

#### `difference(array1, array2) -> array`
Set difference: elements in array1 that are not in array2.

```bash
redis> JSON.SET doc $ '{"a": [1, 2, 3, 4], "b": [2, 4]}'
redis> JSON.JMESPATH doc "difference(a, b)"
"[1,3]"
```

#### `intersection(array1, array2) -> array`
Set intersection: elements present in both arrays.

```bash
redis> JSON.SET doc $ '{"a": [1, 2, 3], "b": [2, 3, 4]}'
redis> JSON.JMESPATH doc "intersection(a, b)"
"[2,3]"
```

#### `union(array1, array2) -> array`
Set union: unique elements from both arrays.

```bash
redis> JSON.SET doc $ '{"a": [1, 2], "b": [2, 3]}'
redis> JSON.JMESPATH doc "union(a, b)"
"[1,2,3]"
```

#### `group_by(array, field) -> object`
Group array of objects by a field value.

```bash
redis> JSON.SET doc $ '[{"name":"Alice","role":"admin"},{"name":"Bob","role":"user"},{"name":"Carol","role":"admin"}]'
redis> JSON.JMESPATH doc "group_by(@, 'role')"
"{\"admin\":[{\"name\":\"Alice\",\"role\":\"admin\"},{\"name\":\"Carol\",\"role\":\"admin\"}],\"user\":[{\"name\":\"Bob\",\"role\":\"user\"}]}"
```

### Object Functions (4)

#### `entries(object) -> array`
Convert object to array of `{key, value}` objects.

```bash
redis> JSON.SET doc $ '{"a": 1, "b": 2}'
redis> JSON.JMESPATH doc "entries(@)"
"[{\"key\":\"a\",\"value\":1},{\"key\":\"b\",\"value\":2}]"
```

#### `from_entries(array) -> object`
Convert array of `{key, value}` objects back to object.

```bash
redis> JSON.SET doc $ '[{"key":"a","value":1},{"key":"b","value":2}]'
redis> JSON.JMESPATH doc "from_entries(@)"
"{\"a\":1,\"b\":2}"
```

#### `pick(object, keys) -> object`
Select only specified keys from an object.

```bash
redis> JSON.SET doc $ '{"name": "Alice", "age": 30, "email": "alice@example.com", "password": "secret"}'
redis> JSON.JMESPATH doc "pick(@, ['name', 'email'])"
"{\"email\":\"alice@example.com\",\"name\":\"Alice\"}"
```

#### `omit(object, keys) -> object`
Exclude specified keys from an object.

```bash
redis> JSON.SET doc $ '{"name": "Alice", "age": 30, "password": "secret"}'
redis> JSON.JMESPATH doc "omit(@, ['password'])"
"{\"age\":30,\"name\":\"Alice\"}"
```

### Math/Statistics Functions (11)

#### `round(number, precision?) -> number`
Round to specified decimal places (default 0).

```bash
redis> JSON.SET doc $ '{"n": 3.14159}'
redis> JSON.JMESPATH doc "round(n)"
"3.0"

redis> JSON.JMESPATH doc "round(n, `2`)"
"3.14"
```

#### `floor_fn(number) -> number`
Round down to nearest integer.

```bash
redis> JSON.SET doc $ '{"n": 3.7}'
redis> JSON.JMESPATH doc "floor_fn(n)"
"3"
```

#### `ceil_fn(number) -> number`
Round up to nearest integer.

```bash
redis> JSON.SET doc $ '{"n": 3.2}'
redis> JSON.JMESPATH doc "ceil_fn(n)"
"4"
```

#### `abs_fn(number) -> number`
Absolute value.

```bash
redis> JSON.SET doc $ '{"n": -5}'
redis> JSON.JMESPATH doc "abs_fn(n)"
"5.0"
```

#### `mod_fn(number, divisor) -> number`
Modulo operation.

```bash
redis> JSON.SET doc $ '{"a": 10, "b": 3}'
redis> JSON.JMESPATH doc "mod_fn(a, b)"
"1.0"
```

#### `pow(base, exponent) -> number`
Exponentiation.

```bash
redis> JSON.SET doc $ '{"base": 2, "exp": 3}'
redis> JSON.JMESPATH doc "pow(base, exp)"
"8.0"
```

#### `sqrt(number) -> number`
Square root.

```bash
redis> JSON.SET doc $ '{"n": 16}'
redis> JSON.JMESPATH doc "sqrt(n)"
"4.0"
```

#### `log(number, base?) -> number`
Logarithm. Default base is e (natural log).

```bash
redis> JSON.SET doc $ '{"n": 100}'
redis> JSON.JMESPATH doc "log(n, `10`)"
"2.0"
```

#### `clamp(number, min, max) -> number`
Constrain number to range.

```bash
redis> JSON.SET doc $ '{"n": 15}'
redis> JSON.JMESPATH doc "clamp(n, `0`, `10`)"
"10.0"
```

#### `median(array) -> number`
Calculate median of numeric array.

```bash
redis> JSON.SET doc $ '{"scores": [1, 3, 5, 7, 9]}'
redis> JSON.JMESPATH doc "median(scores)"
"5.0"

redis> JSON.SET doc $ '{"scores": [1, 2, 3, 4]}'
redis> JSON.JMESPATH doc "median(scores)"
"2.5"
```

#### `percentile(array, p) -> number`
Calculate pth percentile (0-100) using linear interpolation.

```bash
redis> JSON.SET doc $ '{"latencies": [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]}'
redis> JSON.JMESPATH doc "percentile(latencies, `95`)"
"95.5"

redis> JSON.JMESPATH doc "percentile(latencies, `50`)"
"55.0"
```

### Type Functions (10)

#### `to_string(any) -> string`
Convert any value to string representation.

```bash
redis> JSON.SET doc $ '{"n": 42}'
redis> JSON.JMESPATH doc "to_string(n)"
"\"42\""
```

#### `to_number(any) -> number`
Convert to number. Returns null if not convertible.

```bash
redis> JSON.SET doc $ '{"s": "42"}'
redis> JSON.JMESPATH doc "to_number(s)"
"42.0"
```

#### `to_boolean(any) -> boolean`
Convert to boolean using truthy/falsy rules.

```bash
redis> JSON.SET doc $ '{"s": "hello"}'
redis> JSON.JMESPATH doc "to_boolean(s)"
"true"

redis> JSON.SET doc $ '{"s": ""}'
redis> JSON.JMESPATH doc "to_boolean(s)"
"false"
```

#### `type_of(any) -> string`
Get type name: "string", "number", "boolean", "null", "array", "object".

```bash
redis> JSON.SET doc $ '{"a": [1,2,3]}'
redis> JSON.JMESPATH doc "type_of(a)"
"\"array\""
```

#### `is_string(any) -> boolean`
Check if value is a string.

```bash
redis> JSON.SET doc $ '{"s": "hello"}'
redis> JSON.JMESPATH doc "is_string(s)"
"true"
```

#### `is_number(any) -> boolean`
Check if value is a number.

```bash
redis> JSON.SET doc $ '{"n": 42}'
redis> JSON.JMESPATH doc "is_number(n)"
"true"
```

#### `is_boolean(any) -> boolean`
Check if value is a boolean.

```bash
redis> JSON.SET doc $ '{"b": true}'
redis> JSON.JMESPATH doc "is_boolean(b)"
"true"
```

#### `is_array(any) -> boolean`
Check if value is an array.

```bash
redis> JSON.SET doc $ '{"a": [1,2,3]}'
redis> JSON.JMESPATH doc "is_array(a)"
"true"
```

#### `is_object(any) -> boolean`
Check if value is an object.

```bash
redis> JSON.SET doc $ '{"o": {"key": "value"}}'
redis> JSON.JMESPATH doc "is_object(o)"
"true"
```

#### `is_null(any) -> boolean`
Check if value is null.

```bash
redis> JSON.SET doc $ '{"n": null}'
redis> JSON.JMESPATH doc "is_null(n)"
"true"
```

### Hash/Checksum Functions (4)

#### `md5(string) -> string`
Returns hex-encoded MD5 hash of the input string.

```bash
redis> JSON.SET doc $ '{"data": "hello world"}'
redis> JSON.JMESPATH doc "md5(data)"
"\"5eb63bbbe01eeed093cb22bb8f5acdc3\""
```

#### `sha1(string) -> string`
Returns hex-encoded SHA-1 hash of the input string.

```bash
redis> JSON.SET doc $ '{"data": "hello world"}'
redis> JSON.JMESPATH doc "sha1(data)"
"\"2aae6c35c94fcfb415dbe95f408b9ce91ee846ed\""
```

#### `sha256(string) -> string`
Returns hex-encoded SHA-256 hash of the input string.

```bash
redis> JSON.SET doc $ '{"data": "hello world"}'
redis> JSON.JMESPATH doc "sha256(data)"
"\"b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9\""
```

#### `crc32(string) -> number`
Returns CRC32 checksum as an integer.

```bash
redis> JSON.SET doc $ '{"data": "hello world"}'
redis> JSON.JMESPATH doc "crc32(data)"
"222957957"
```

### Utility/Conditional Functions (4)

#### `now() -> number`
Returns current Unix timestamp in seconds.

```bash
redis> JSON.SET doc $ '{"expires": 1700000000}'
redis> JSON.JMESPATH doc "expires > now()"
"true"  # or "false" depending on current time
```

#### `now_ms() -> number`
Returns current Unix timestamp in milliseconds.

```bash
redis> JSON.SET doc $ '{}'
redis> JSON.JMESPATH doc "now_ms()"
"1733417234567"  # example output
```

#### `default(value, fallback) -> value`
Return fallback if value is null, otherwise return value.

```bash
redis> JSON.SET doc $ '{"name": null, "role": "admin"}'
redis> JSON.JMESPATH doc "default(name, 'Unknown')"
"\"Unknown\""

redis> JSON.JMESPATH doc "default(role, 'guest')"
"\"admin\""
```

#### `if(condition, then, else) -> any`
Ternary conditional. Returns `then` if condition is truthy, otherwise `else`.
In JMESPath, only `false` and `null` are falsy; everything else (including 0 and empty strings) is truthy.

```bash
redis> JSON.SET doc $ '{"age": 25}'
redis> JSON.JMESPATH doc "if(age >= `18`, 'adult', 'minor')"
"\"adult\""

# Nested conditionals for grade calculation
redis> JSON.SET doc $ '{"score": 85}'
redis> JSON.JMESPATH doc "if(score >= `90`, 'A', if(score >= `80`, 'B', if(score >= `70`, 'C', 'F')))"
"\"B\""

# With comparison expressions
redis> JSON.SET doc $ '{"items": [1, 2, 3]}'
redis> JSON.JMESPATH doc "if(length(items) > `0`, first(items), 'empty')"
"1"
```

---

## Advanced Examples

### Data Transformation Pipeline

```bash
# Original data
redis> JSON.SET orders $ '[
  {"id": "ORD-001", "items": [{"name": "Widget", "qty": 2, "price": 10}], "status": "shipped"},
  {"id": "ORD-002", "items": [{"name": "Gadget", "qty": 1, "price": 25}], "status": "pending"},
  {"id": "ORD-003", "items": [{"name": "Widget", "qty": 5, "price": 10}], "status": "shipped"}
]'

# Get shipped orders with totals
redis> JSON.JMESPATH orders "[?status == 'shipped'].{
  order_id: id,
  total: sum(items[*].price)
}"
"[{\"order_id\":\"ORD-001\",\"total\":10},{\"order_id\":\"ORD-003\",\"total\":10}]"

# Get unique product names
redis> JSON.JMESPATH orders "unique([*].items[*].name[])"
"[\"Widget\",\"Gadget\"]"
```

### Working with Nested Data

```bash
redis> JSON.SET company $ '{
  "departments": [
    {"name": "Engineering", "employees": [{"name": "Alice", "level": 3}, {"name": "Bob", "level": 2}]},
    {"name": "Sales", "employees": [{"name": "Carol", "level": 2}]}
  ]
}'

# Get all employee names
redis> JSON.JMESPATH company "departments[*].employees[*].name[]"
"[\"Alice\",\"Bob\",\"Carol\"]"

# Find senior engineers (level >= 3)
redis> JSON.JMESPATH company "departments[?name == 'Engineering'].employees[?level >= `3`][].name"
"[\"Alice\"]"

# Count employees per department
redis> JSON.JMESPATH company "departments[*].{dept: name, count: length(employees)}"
"[{\"dept\":\"Engineering\",\"count\":2},{\"dept\":\"Sales\",\"count\":1}]"
```

### Combining Custom Functions

```bash
redis> JSON.SET data $ '{
  "users": [
    {"name": "  ALICE  ", "tags": "admin,power-user"},
    {"name": "  bob  ", "tags": "user,guest"}
  ]
}'

# Normalize names and split tags
redis> JSON.JMESPATH data "users[*].{
  name: lower(trim(name)),
  roles: split(tags, ',')
}"
"[{\"name\":\"alice\",\"roles\":[\"admin\",\"power-user\"]},{\"name\":\"bob\",\"roles\":[\"user\",\"guest\"]}]"

# Get unique roles across all users
redis> JSON.JMESPATH data "unique(users[*].tags | [].split(@, ',')[])"
"[\"admin\",\"power-user\",\"user\",\"guest\"]"
```

### Math and Type Functions

```bash
redis> JSON.SET items $ '[
  {"name": "A", "price": 10.567},
  {"name": "B", "price": 20.123},
  {"name": "C", "price": 15.999}
]'

# Round prices and filter
redis> JSON.JMESPATH items "[*].{name: name, price: round(price, `2`)}"
"[{\"name\":\"A\",\"price\":10.57},{\"name\":\"B\",\"price\":20.12},{\"name\":\"C\",\"price\":16.0}]"

# Filter by type
redis> JSON.SET mixed $ '{"items": [1, "hello", true, null, [1,2], {"a": 1}]}'
redis> JSON.JMESPATH mixed "items[?is_number(@)]"
"[1]"

redis> JSON.JMESPATH mixed "items[?is_string(@)]"
"[\"hello\"]"
```

### Generate and Transform

```bash
# Generate a sequence
redis> JSON.SET doc $ '{}'
redis> JSON.JMESPATH doc "range(`1`, `6`)"
"[1,2,3,4,5]"

# Paginate results
redis> JSON.SET items $ '{"data": [1,2,3,4,5,6,7,8,9,10]}'
redis> JSON.JMESPATH items "data | drop(@, `3`) | take(@, `3`)"
"[4,5,6]"
```

---

## RESP3 Native Output

When using RESP3 protocol, you can get native Redis types instead of JSON strings:

```bash
# Default: JSON string
redis> JSON.JMESPATH users "[*].name"
"[\"Alice\",\"Bob\"]"

# With FORMAT EXPAND: Native RESP3 array
redis> JSON.JMESPATH users "[*].name" FORMAT EXPAND
1) "Alice"
2) "Bob"

# Objects become RESP3 maps
redis> JSON.JMESPATH users "[0]" FORMAT EXPAND
1# "name" => "Alice"
2# "age" => (integer) 30
3# "role" => "admin"
```

---

## Performance Considerations

### Expression Caching

Compiled JMESPath expressions are cached per-thread using an LRU cache (256 entries). Repeated queries with the same expression skip the parsing/compilation phase entirely.

**Benchmark results** (Rust criterion benchmarks):

| Operation | Time |
|-----------|------|
| Cache hit (lookup) | ~15ns |
| Cache miss (simple expression) | ~77-173ns |
| Cache miss (complex expression) | ~400-530ns |

This means cached expressions are **10-35x faster** than re-parsing on every call. The cache is especially effective for:
- Hot paths with repeated queries
- Application patterns that reuse expressions with different keys  
- High-throughput read workloads

**End-to-end throughput** (Python benchmarks against live Redis):

| Query Type | Ops/sec |
|------------|---------|
| Simple field access | ~3,300 |
| Array projection (100 items) | ~2,600 |
| Filter + project (100 items) | ~2,500 |
| Complex (filter + sort + slice) | ~1,600 |

### JMESPath vs JSONPath Performance

For equivalent operations, JMESPath performs comparably to JSONPath:

| Operation | JMESPath/JSONPath Ratio |
|-----------|------------------------|
| Simple field access | ~1.0x |
| Array projection | ~1.2x |
| Filter expressions | ~1.2x |

The slight overhead is offset by JMESPath's ability to do complex transformations in a single command (avoiding multiple round trips).

### Optimization Tips

1. **Large Documents**: JMESPath evaluates against the full document in memory. For very large documents, consider storing data in smaller chunks.

2. **Filter Early**: Deeply nested projections and multiple filters can impact performance. Optimize by filtering early in the pipeline:
   ```jmespath
   # Good: filter first, then project
   items[?status == 'active'] | [*].{id: id, name: name}
   
   # Less efficient: project everything, then filter
   items[*].{id: id, name: name, status: status} | [?status == 'active']
   ```

3. **Range Limits**: The `range()` function is limited to 10,000 elements to prevent runaway allocations.

### Running Benchmarks

```bash
# Python end-to-end benchmarks
python3 util/jmespath_benchmark.py -p 6379 -n 1000

# Rust micro-benchmarks
cargo bench --features jmespath -p redis_json --bench jmespath_cache
```

---

## Portability

### Portable Queries (Standard JMESPath)
These work in any JMESPath implementation (Python, JavaScript, Go, etc.):

```jmespath
users[?age > `18`].name
sort_by(items, &price) | [0]
{total: sum(prices), count: length(items)}
```

### Redis-Specific Queries (Custom Functions)
These only work in RedisJSON:

```jmespath
users[?lower(name) == 'alice']
unique(items[*].category)
default(config.timeout, `30`)
range(`0`, `10`) | map(&pow(@, `2`), @)
items[?is_number(@)]
```

---

## Function Quick Reference

### Standard Functions (26)
`abs`, `avg`, `ceil`, `contains`, `ends_with`, `floor`, `join`, `keys`, `length`, `map`, `max`, `max_by`, `merge`, `min`, `min_by`, `not_null`, `reverse`, `sort`, `sort_by`, `starts_with`, `sum`, `to_array`, `to_number`, `to_string`, `type`, `values`

### Custom String Functions (15)
`lower`, `upper`, `trim`, `capitalize`, `title`, `split`, `replace`, `repeat`, `pad_left`, `pad_right`, `substr`, `slice`, `index_of`, `last_index_of`, `concat`

### Custom Array Functions (17)
`unique`, `zip`, `chunk`, `take`, `drop`, `flatten_deep`, `compact`, `range`, `index_at`, `includes`, `find_index`, `first`, `last`, `difference`, `intersection`, `union`, `group_by`

### Custom Object Functions (4)
`entries`, `from_entries`, `pick`, `omit`

### Custom Math/Statistics Functions (11)
`round`, `floor_fn`, `ceil_fn`, `abs_fn`, `mod_fn`, `pow`, `sqrt`, `log`, `clamp`, `median`, `percentile`

### Custom Type Functions (10)
`to_string`, `to_number`, `to_boolean`, `type_of`, `is_string`, `is_number`, `is_boolean`, `is_array`, `is_object`, `is_null`

### Custom Utility/Conditional Functions (4)
`now`, `now_ms`, `default`, `if`

### Custom Hash/Checksum Functions (4)
`md5`, `sha1`, `sha256`, `crc32`

---

## Error Handling

| Error | Cause |
|-------|-------|
| `ERR JMESPath compile error` | Invalid expression syntax |
| `ERR JMESPath error` | Runtime evaluation error |
| `ERR FORMAT argument is not supported on RESP2` | FORMAT used without RESP3 |
| `Division by zero` | `mod_fn` with zero divisor |
| `Cannot take square root of negative number` | `sqrt` with negative input |
| `Logarithm requires positive number` | `log` with non-positive input |
| `Step cannot be zero` | `range` with zero step |

---

## Future Function Ideas

The following functions are being considered for future implementation. Contributions welcome!

### String Functions
| Function | Description | Example |
|----------|-------------|---------|
| `trim_start(s)` | Remove leading whitespace only | `trim_start("  hi")` → `"hi"` |
| `trim_end(s)` | Remove trailing whitespace only | `trim_end("hi  ")` → `"hi"` |
| `camel_case(s)` | Convert to camelCase | `camel_case("hello_world")` → `"helloWorld"` |
| `snake_case(s)` | Convert to snake_case | `snake_case("helloWorld")` → `"hello_world"` |
| `kebab_case(s)` | Convert to kebab-case | `kebab_case("helloWorld")` → `"hello-world"` |
| `truncate(s, len, suffix?)` | Truncate with ellipsis | `truncate(title, 20, "...")` |
| `wrap(s, width)` | Word-wrap text | `wrap(description, 80)` |
| `match(s, regex)` | Regex match (returns bool) | `match(email, "^.+@.+$")` |
| `extract(s, regex)` | Extract regex groups | `extract(url, "https?://([^/]+)")` |
| `format(template, ...)` | String interpolation | `format("{0} is {1}", name, age)` |

### Array Functions
| Function | Description | Example |
|----------|-------------|---------|
| `nth(arr, n)` | Every nth element | `nth(items, 2)` → every 2nd |
| `interleave(arr1, arr2)` | Alternate elements | `interleave([1,2], [a,b])` → `[1,a,2,b]` |
| `partition(arr, size)` | Split into n equal parts | `partition(items, 3)` |
| `rotate(arr, n)` | Rotate elements | `rotate([1,2,3], 1)` → `[2,3,1]` |
| `shuffle(arr, seed?)` | Deterministic shuffle | `shuffle(items, 42)` |
| `sample(arr, n, seed?)` | Random sample | `sample(items, 5)` |
| `frequencies(arr)` | Count occurrences | `frequencies(tags)` → `{a: 2, b: 1}` |
| `index_by(arr, &expr)` | Create lookup object | `index_by(users, &id)` |
| `cartesian(arr1, arr2)` | Cartesian product | `cartesian([1,2], [a,b])` |

### Object Functions
| Function | Description | Example |
|----------|-------------|---------|
| `rename_keys(obj, map)` | Rename object keys | `rename_keys(obj, {old: 'new'})` |
| `deep_merge(obj1, obj2)` | Recursive merge | `deep_merge(defaults, config)` |
| `flatten_keys(obj, sep?)` | Flatten nested object | `flatten_keys({a: {b: 1}})` → `{"a.b": 1}` |
| `unflatten_keys(obj, sep?)` | Unflatten object | `unflatten_keys({"a.b": 1})` → `{a: {b: 1}}` |
| `map_values(obj, &expr)` | Transform all values | `map_values(prices, &round(@, 2))` |
| `map_keys(obj, &expr)` | Transform all keys | `map_keys(obj, &lower(@))` |
| `filter_keys(obj, &expr)` | Filter by key | `filter_keys(obj, &starts_with(@, 'user_'))` |
| `filter_values(obj, &expr)` | Filter by value | `filter_values(obj, &@ != null)` |
| `invert(obj)` | Swap keys and values | `invert({a: 1, b: 2})` → `{1: "a", 2: "b"}` |

### Math/Statistics Functions
| Function | Description | Example |
|----------|-------------|---------|
| `mode(arr)` | Most frequent value | `mode(ratings)` |
| `stddev(arr)` | Standard deviation | `stddev(measurements)` |
| `variance(arr)` | Variance | `variance(measurements)` |
| `sin(n)`, `cos(n)`, `tan(n)` | Trigonometry | `sin(angle)` |
| `random(min?, max?)` | Random number | `random(1, 100)` |
| `sign(n)` | Sign (-1, 0, 1) | `sign(balance)` |

### Date/Time Functions
| Function | Description | Example |
|----------|-------------|---------|
| `parse_date(s, format?)` | Parse to timestamp | `parse_date("2024-01-15", "YYYY-MM-DD")` |
| `format_date(ts, format)` | Format timestamp | `format_date(created_at, "YYYY-MM-DD")` |
| `date_add(ts, amount, unit)` | Add to date | `date_add(now(), 7, 'days')` |
| `date_diff(ts1, ts2, unit)` | Difference | `date_diff(end, start, 'hours')` |
| `date_part(ts, part)` | Extract component | `date_part(ts, 'year')` |
| `start_of(ts, unit)` | Start of period | `start_of(ts, 'month')` |
| `end_of(ts, unit)` | End of period | `end_of(ts, 'week')` |

### Encoding Functions
| Function | Description | Example |
|----------|-------------|---------|
| `base64_encode(s)` | Encode to base64 | `base64_encode(data)` |
| `base64_decode(s)` | Decode from base64 | `base64_decode(encoded)` |
| `url_encode(s)` | URL encode | `url_encode(query)` |
| `url_decode(s)` | URL decode | `url_decode(param)` |
| `json_encode(any)` | Encode as JSON string | `json_encode(obj)` |
| `json_decode(s)` | Parse JSON string | `json_decode(json_str)` |
| `hex_encode(s)` | Encode to hex | `hex_encode(data)` |
| `hex_decode(s)` | Decode from hex | `hex_decode(hex_str)` |

### Hash/Crypto Functions
| Function | Description | Example |
|----------|-------------|---------|
| `uuid()` | Generate UUID v4 | `uuid()` |

### Conditional/Logic Functions
| Function | Description | Example |
|----------|-------------|---------|
| `coalesce(...)` | First non-null (variadic) | `coalesce(a, b, c, 'default')` |
| `switch(val, cases, default)` | Switch/case | `switch(status, {1: 'ok', 2: 'err'}, 'unknown')` |
| `all(arr, &expr)` | All match predicate | `all(items, &@ > 0)` |
| `any(arr, &expr)` | Any matches predicate | `any(items, &@ > 100)` |
| `none(arr, &expr)` | None match predicate | `none(items, &is_null(@))` |

### Path/URL Functions
| Function | Description | Example |
|----------|-------------|---------|
| `path_join(...)` | Join path segments | `path_join(dir, subdir, file)` |
| `path_basename(p)` | Get filename | `path_basename("/a/b/c.txt")` → `"c.txt"` |
| `path_dirname(p)` | Get directory | `path_dirname("/a/b/c.txt")` → `"/a/b"` |
| `path_ext(p)` | Get extension | `path_ext("file.json")` → `".json"` |
| `url_parse(url)` | Parse URL components | `url_parse(link).host` |

### Validation Functions
| Function | Description | Example |
|----------|-------------|---------|
| `is_email(s)` | Valid email format | `is_email(contact)` |
| `is_url(s)` | Valid URL format | `is_url(link)` |
| `is_uuid(s)` | Valid UUID format | `is_uuid(id)` |
| `is_ip(s)` | Valid IP address | `is_ip(addr)` |
| `is_json(s)` | Valid JSON string | `is_json(payload)` |
| `is_empty(x)` | Empty string/array/object | `is_empty(results)` |
| `is_blank(s)` | Empty or whitespace only | `is_blank(input)` |

### Implementation Notes

Some functions face technical limitations:

- **Expression-based functions** (`group_by`, `all`, `any`, etc.): Require access to the JMESPath interpreter, which is not publicly exposed by the `jmespath` crate. Would require forking the crate or upstream changes.

- **Regex functions** (`match`, `extract`): Would add the `regex` crate as a dependency. Consider feature-gating.

- **Crypto functions**: Would add crypto dependencies. Consider feature-gating for security-conscious deployments.

- **Date functions**: Would benefit from the `chrono` crate for robust parsing/formatting.

- **Random functions**: Need to consider determinism requirements for replication.

---

## Feature Flag

JMESPath support can be disabled at compile time:

```toml
# In Cargo.toml - disable JMESPath
[dependencies]
redis_json = { version = "...", default-features = false }

# Or explicitly enable
redis_json = { version = "...", features = ["jmespath"] }
```

When disabled, the `JSON.JMESPATH` command is not registered and the jmespath crate is not compiled.

---

## See Also

- [JMESPath Specification](https://jmespath.org/specification.html)
- [JMESPath Tutorial](https://jmespath.org/tutorial.html)
- [JMESPath Examples](https://jmespath.org/examples.html)
