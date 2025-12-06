# JMESPath Support in RedisJSON

> **TL;DR:** `JSON.JMESPATH` provides JMESPath query support for RedisJSON - a powerful read-only query language with 26 standard + 129 custom functions for data extraction and transformation. Compiled expressions are cached for performance.

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
| **Functions** | 26 built-in + 129 custom | Limited |
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

## Custom Redis Functions (129)

These functions extend JMESPath with capabilities specific to RedisJSON. **Note:** Queries using these functions are not portable to other JMESPath implementations.

### String Functions (27)

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

#### `truncate(string, length, suffix?) -> string`
Truncate a string to the specified length, adding a suffix (default "...") if truncated.

```bash
redis> JSON.SET doc $ '{"s": "hello world"}'
redis> JSON.JMESPATH doc "truncate(s, `5`)"
"\"he...\""

# Custom suffix
redis> JSON.JMESPATH doc "truncate(s, `8`, `\"--\"`)"
"\"hello --\""

# No truncation needed
redis> JSON.JMESPATH doc "truncate(s, `50`)"
"\"hello world\""
```

#### `trim_start(string) -> string`
Remove leading whitespace from a string.

```bash
redis> JSON.SET doc $ '{"s": "   hello"}'
redis> JSON.JMESPATH doc "trim_start(s)"
"\"hello\""
```

#### `trim_end(string) -> string`
Remove trailing whitespace from a string.

```bash
redis> JSON.SET doc $ '{"s": "hello   "}'
redis> JSON.JMESPATH doc "trim_end(s)"
"\"hello\""
```

#### `regex_match(string, pattern) -> boolean`
Check if a string matches a regular expression pattern. Returns null if the pattern is invalid.

```bash
redis> JSON.SET doc $ '{"email": "user@example.com"}'
redis> JSON.JMESPATH doc "regex_match(email, '^[^@]+@[^@]+\\.[^@]+$')"
"true"

redis> JSON.SET doc $ '{"phone": "555-1234"}'
redis> JSON.JMESPATH doc "regex_match(phone, '^\\d{3}-\\d{4}$')"
"true"
```

#### `regex_extract(string, pattern) -> array`
Extract all matches from a string using a regular expression. Returns an array of matches, or null if the pattern is invalid.

```bash
redis> JSON.SET doc $ '{"text": "Call 555-1234 or 555-5678"}'
redis> JSON.JMESPATH doc "regex_extract(text, '\\d{3}-\\d{4}')"
"[\"555-1234\",\"555-5678\"]"

redis> JSON.SET doc $ '{"log": "ERROR: file not found, ERROR: access denied"}'
redis> JSON.JMESPATH doc "regex_extract(log, 'ERROR: [^,]+')"
"[\"ERROR: file not found\",\"ERROR: access denied\"]"
```

#### `regex_replace(string, pattern, replacement) -> string`
Replace all matches of a regular expression with a replacement string. Returns null if the pattern is invalid.

```bash
redis> JSON.SET doc $ '{"text": "Hello World"}'
redis> JSON.JMESPATH doc "regex_replace(text, 'World', 'Redis')"
"\"Hello Redis\""

redis> JSON.SET doc $ '{"phone": "555-123-4567"}'
redis> JSON.JMESPATH doc "regex_replace(phone, '-', '.')"
"\"555.123.4567\""

# Redact sensitive data
redis> JSON.SET doc $ '{"data": "SSN: 123-45-6789"}'
redis> JSON.JMESPATH doc "regex_replace(data, '\\d{3}-\\d{2}-\\d{4}', 'XXX-XX-XXXX')"
"\"SSN: XXX-XX-XXXX\""
```

#### `wrap(string, width) -> string`
Word-wrap text at the specified width, preserving words when possible.

```bash
redis> JSON.SET doc $ '{"text": "The quick brown fox jumps over the lazy dog"}'
redis> JSON.JMESPATH doc "wrap(text, `20`)"
"\"The quick brown fox\\njumps over the lazy\\ndog\""

redis> JSON.SET doc $ '{"desc": "Short"}'
redis> JSON.JMESPATH doc "wrap(desc, `80`)"
"\"Short\""
```

#### `format(template, ...args) -> string`
String interpolation using `{0}`, `{1}`, etc. as placeholders. Supports variadic arguments.

```bash
redis> JSON.SET doc $ '{"name": "Alice", "age": 30}'
redis> JSON.JMESPATH doc "format('{0} is {1} years old', name, age)"
"\"Alice is 30 years old\""

redis> JSON.SET doc $ '{"x": 10, "y": 20}'
redis> JSON.JMESPATH doc "format('Point({0}, {1})', x, y)"
"\"Point(10, 20)\""

redis> JSON.SET doc $ '{"user": "bob", "action": "login", "time": "10:30"}'
redis> JSON.JMESPATH doc "format('[{2}] {0}: {1}', user, action, time)"
"\"[10:30] bob: login\""
```

### Array Functions (26)

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

#### `frequencies(array) -> object`
Count occurrences of each value in an array.

```bash
redis> JSON.SET doc $ '{"tags": ["redis", "json", "redis", "nosql", "json", "redis"]}'
redis> JSON.JMESPATH doc "frequencies(tags)"
"{\"json\":2,\"nosql\":1,\"redis\":3}"

# Works with numbers too
redis> JSON.SET doc $ '{"scores": [1, 2, 1, 3, 2, 1]}'
redis> JSON.JMESPATH doc "frequencies(scores)"
"{\"1\":3,\"2\":2,\"3\":1}"
```

#### `nth(array, n) -> array`
Select every nth element from an array.

```bash
redis> JSON.SET doc $ '{"arr": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]}'
redis> JSON.JMESPATH doc "nth(arr, `2`)"
"[1,3,5,7,9]"

redis> JSON.JMESPATH doc "nth(arr, `3`)"
"[1,4,7,10]"
```

#### `interleave(array1, array2) -> array`
Alternate elements from two arrays. Remaining elements from the longer array are appended.

```bash
redis> JSON.SET doc $ '{"a": [1, 2, 3], "b": ["a", "b", "c"]}'
redis> JSON.JMESPATH doc "interleave(a, b)"
"[1,\"a\",2,\"b\",3,\"c\"]"

redis> JSON.SET doc $ '{"a": [1, 2], "b": ["x", "y", "z", "w"]}'
redis> JSON.JMESPATH doc "interleave(a, b)"
"[1,\"x\",2,\"y\",\"z\",\"w\"]"
```

#### `rotate(array, n) -> array`
Rotate array elements by n positions. Positive n rotates left, negative rotates right.

```bash
redis> JSON.SET doc $ '{"arr": [1, 2, 3, 4, 5]}'
redis> JSON.JMESPATH doc "rotate(arr, `2`)"
"[3,4,5,1,2]"

redis> JSON.JMESPATH doc "rotate(arr, `-1`)"
"[5,1,2,3,4]"
```

#### `partition(array, n) -> array`
Split array into n equal-sized parts. Last part may be smaller.

```bash
redis> JSON.SET doc $ '{"arr": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]}'
redis> JSON.JMESPATH doc "partition(arr, `3`)"
"[[1,2,3,4],[5,6,7],[8,9,10]]"

redis> JSON.JMESPATH doc "partition(arr, `4`)"
"[[1,2,3],[4,5,6],[7,8],[9,10]]"
```

#### `shuffle(array, seed?) -> array`
Shuffle array elements. Optionally provide a seed for deterministic shuffling (important for replication).

```bash
redis> JSON.SET doc $ '{"arr": [1, 2, 3, 4, 5]}'
# Deterministic shuffle with seed
redis> JSON.JMESPATH doc "shuffle(arr, `42`)"
"[3,5,1,2,4]"

# Same seed produces same result
redis> JSON.JMESPATH doc "shuffle(arr, `42`)"
"[3,5,1,2,4]"

# Different seed produces different result
redis> JSON.JMESPATH doc "shuffle(arr, `123`)"
"[2,4,1,5,3]"
```

**Note:** When using without a seed, results are non-deterministic and may differ across replicas.

#### `sample(array, n, seed?) -> array`
Randomly select n elements from an array without replacement. Optionally provide a seed for deterministic sampling.

```bash
redis> JSON.SET doc $ '{"arr": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]}'
# Deterministic sample with seed
redis> JSON.JMESPATH doc "sample(arr, `3`, `42`)"
"[6,1,4]"

# Same seed produces same result
redis> JSON.JMESPATH doc "sample(arr, `3`, `42`)"
"[6,1,4]"

# Requesting more elements than available returns all elements (shuffled)
redis> JSON.JMESPATH doc "sample(arr, `20`, `42`)"
"[6,1,4,2,10,3,7,5,9,8]"
```

**Note:** When using without a seed, results are non-deterministic and may differ across replicas.

#### `cartesian(array1, array2) -> array`
Compute the Cartesian product of two arrays. Returns an array of pairs.

```bash
redis> JSON.SET doc $ '{"a": [1, 2], "b": ["x", "y"]}'
redis> JSON.JMESPATH doc "cartesian(a, b)"
"[[1,\"x\"],[1,\"y\"],[2,\"x\"],[2,\"y\"]]"

redis> JSON.SET doc $ '{"sizes": ["S", "M", "L"], "colors": ["red", "blue"]}'
redis> JSON.JMESPATH doc "cartesian(sizes, colors)"
"[[\"S\",\"red\"],[\"S\",\"blue\"],[\"M\",\"red\"],[\"M\",\"blue\"],[\"L\",\"red\"],[\"L\",\"blue\"]]"
```

### Object Functions (9)

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

#### `deep_merge(object1, object2) -> object`
Recursively merge two objects. Values from the second object override the first, but nested objects are merged recursively.

```bash
redis> JSON.SET doc $ '{
  "defaults": {"debug": false, "port": 8080, "db": {"host": "localhost", "port": 5432}},
  "overrides": {"debug": true, "db": {"port": 5433}}
}'
redis> JSON.JMESPATH doc "deep_merge(defaults, overrides)"
"{\"db\":{\"host\":\"localhost\",\"port\":5433},\"debug\":true,\"port\":8080}"

# Simple merge
redis> JSON.SET doc $ '{"a": {"x": 1}, "b": {"y": 2}}'
redis> JSON.JMESPATH doc "deep_merge(a, b)"
"{\"x\":1,\"y\":2}"
```

#### `invert(object) -> object`
Swap keys and values. Values must be strings or numbers.

```bash
redis> JSON.SET doc $ '{"a": "1", "b": "2", "c": "3"}'
redis> JSON.JMESPATH doc "invert(@)"
"{\"1\":\"a\",\"2\":\"b\",\"3\":\"c\"}"

redis> JSON.SET doc $ '{"red": "#ff0000", "green": "#00ff00"}'
redis> JSON.JMESPATH doc "invert(@)"
"{\"#00ff00\":\"green\",\"#ff0000\":\"red\"}"
```

#### `rename_keys(object, mapping) -> object`
Rename object keys according to a mapping object. Keys not in the mapping are preserved.

```bash
redis> JSON.SET doc $ '{"old_name": "Alice", "old_age": 30}'
redis> JSON.JMESPATH doc "rename_keys(@, {old_name: 'name', old_age: 'age'})"
"{\"age\":30,\"name\":\"Alice\"}"

redis> JSON.SET doc $ '{"firstName": "Bob", "lastName": "Smith", "email": "bob@example.com"}'
redis> JSON.JMESPATH doc "rename_keys(@, {firstName: 'first_name', lastName: 'last_name'})"
"{\"email\":\"bob@example.com\",\"first_name\":\"Bob\",\"last_name\":\"Smith\"}"
```

#### `flatten_keys(object, separator?) -> object`
Flatten a nested object into a single-level object with compound keys. Separator defaults to ".".

```bash
redis> JSON.SET doc $ '{"user": {"name": "Alice", "address": {"city": "NYC", "zip": "10001"}}}'
redis> JSON.JMESPATH doc "flatten_keys(@)"
"{\"user.address.city\":\"NYC\",\"user.address.zip\":\"10001\",\"user.name\":\"Alice\"}"

redis> JSON.JMESPATH doc "flatten_keys(@, '/')"
"{\"user/address/city\":\"NYC\",\"user/address/zip\":\"10001\",\"user/name\":\"Alice\"}"
```

#### `unflatten_keys(object, separator?) -> object`
Expand a flat object with compound keys into a nested object. Separator defaults to ".".

```bash
redis> JSON.SET doc $ '{"user.name": "Alice", "user.address.city": "NYC"}'
redis> JSON.JMESPATH doc "unflatten_keys(@)"
"{\"user\":{\"address\":{\"city\":\"NYC\"},\"name\":\"Alice\"}}"

redis> JSON.SET doc $ '{"user/name": "Bob", "user/email": "bob@example.com"}'
redis> JSON.JMESPATH doc "unflatten_keys(@, '/')"
"{\"user\":{\"email\":\"bob@example.com\",\"name\":\"Bob\"}}"
```

### Math/Statistics Functions (23)

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

#### `sin(number) -> number`
Calculate sine of angle in radians.

```bash
redis> JSON.SET doc $ '{"angle": 0}'
redis> JSON.JMESPATH doc "sin(angle)"
"0.0"

redis> JSON.SET doc $ '{"angle": 1.5707963267948966}'  # pi/2
redis> JSON.JMESPATH doc "sin(angle)"
"1.0"
```

#### `cos(number) -> number`
Calculate cosine of angle in radians.

```bash
redis> JSON.SET doc $ '{"angle": 0}'
redis> JSON.JMESPATH doc "cos(angle)"
"1.0"

redis> JSON.SET doc $ '{"angle": 3.141592653589793}'  # pi
redis> JSON.JMESPATH doc "cos(angle)"
"-1.0"
```

#### `tan(number) -> number`
Calculate tangent of angle in radians.

```bash
redis> JSON.SET doc $ '{"angle": 0}'
redis> JSON.JMESPATH doc "tan(angle)"
"0.0"

redis> JSON.SET doc $ '{"angle": 0.7853981633974483}'  # pi/4
redis> JSON.JMESPATH doc "tan(angle)"
"1.0"
```

#### `asin(number) -> number`
Calculate arc sine (inverse sine). Input must be between -1 and 1, returns radians. Returns null for out-of-domain inputs.

```bash
redis> JSON.SET doc $ '{"val": 0}'
redis> JSON.JMESPATH doc "asin(val)"
"0.0"

redis> JSON.SET doc $ '{"val": 1}'
redis> JSON.JMESPATH doc "asin(val)"
"1.5707963267948966"  # pi/2

redis> JSON.SET doc $ '{"val": 2}'  # out of domain
redis> JSON.JMESPATH doc "asin(val)"
"null"
```

#### `acos(number) -> number`
Calculate arc cosine (inverse cosine). Input must be between -1 and 1, returns radians. Returns null for out-of-domain inputs.

```bash
redis> JSON.SET doc $ '{"val": 1}'
redis> JSON.JMESPATH doc "acos(val)"
"0.0"

redis> JSON.SET doc $ '{"val": 0}'
redis> JSON.JMESPATH doc "acos(val)"
"1.5707963267948966"  # pi/2
```

#### `atan(number) -> number`
Calculate arc tangent (inverse tangent). Returns radians.

```bash
redis> JSON.SET doc $ '{"val": 0}'
redis> JSON.JMESPATH doc "atan(val)"
"0.0"

redis> JSON.SET doc $ '{"val": 1}'
redis> JSON.JMESPATH doc "atan(val)"
"0.7853981633974483"  # pi/4
```

#### `sign(number) -> number`
Return the sign of a number: -1 for negative, 0 for zero, 1 for positive.

```bash
redis> JSON.SET doc $ '{"pos": 42, "neg": -17, "zero": 0}'
redis> JSON.JMESPATH doc "sign(pos)"
"1"

redis> JSON.JMESPATH doc "sign(neg)"
"-1"

redis> JSON.JMESPATH doc "sign(zero)"
"0"
```

#### `atan2(y, x) -> number`
Two-argument arctangent. Returns the angle in radians between the positive x-axis and the point (x, y).

```bash
redis> JSON.SET doc $ '{"y": 1, "x": 1}'
redis> JSON.JMESPATH doc "atan2(y, x)"
"0.7853981633974483"  # pi/4

redis> JSON.SET doc $ '{"y": 0, "x": -1}'
redis> JSON.JMESPATH doc "atan2(y, x)"
"3.141592653589793"  # pi
```

#### `deg_to_rad(number) -> number`
Convert degrees to radians.

```bash
redis> JSON.SET doc $ '{"angle": 180}'
redis> JSON.JMESPATH doc "deg_to_rad(angle)"
"3.141592653589793"

redis> JSON.SET doc $ '{"angle": 90}'
redis> JSON.JMESPATH doc "deg_to_rad(angle)"
"1.5707963267948966"
```

#### `rad_to_deg(number) -> number`
Convert radians to degrees.

```bash
redis> JSON.SET doc $ '{"angle": 3.141592653589793}'
redis> JSON.JMESPATH doc "rad_to_deg(angle)"
"180.0"

redis> JSON.SET doc $ '{"angle": 1.5707963267948966}'
redis> JSON.JMESPATH doc "rad_to_deg(angle)"
"90.0"
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

### Encoding Functions (6)

#### `base64_encode(string) -> string`
Encode a string to base64.

```bash
redis> JSON.SET doc $ '{"message": "Hello, World!"}'
redis> JSON.JMESPATH doc "base64_encode(message)"
"\"SGVsbG8sIFdvcmxkIQ==\""

# Encode sensitive data before transmission
redis> JSON.SET doc $ '{"credentials": "user:pass"}'
redis> JSON.JMESPATH doc "base64_encode(credentials)"
"\"dXNlcjpwYXNz\""
```

#### `base64_decode(string) -> string`
Decode a base64-encoded string.

```bash
redis> JSON.SET doc $ '{"encoded": "SGVsbG8sIFdvcmxkIQ=="}'
redis> JSON.JMESPATH doc "base64_decode(encoded)"
"\"Hello, World!\""

# Roundtrip example
redis> JSON.SET doc $ '{"data": "secret"}'
redis> JSON.JMESPATH doc "base64_decode(base64_encode(data))"
"\"secret\""
```

#### `hex_encode(string) -> string`
Encode a string to hexadecimal.

```bash
redis> JSON.SET doc $ '{"data": "hello"}'
redis> JSON.JMESPATH doc "hex_encode(data)"
"\"68656c6c6f\""
```

#### `hex_decode(string) -> string`
Decode a hexadecimal string. Returns null for invalid hex input.

```bash
redis> JSON.SET doc $ '{"hex": "68656c6c6f"}'
redis> JSON.JMESPATH doc "hex_decode(hex)"
"\"hello\""

# Invalid hex returns null
redis> JSON.SET doc $ '{"hex": "not-hex"}'
redis> JSON.JMESPATH doc "hex_decode(hex)"
"null"
```

#### `json_encode(any) -> string`
Encode any value as a JSON string.

```bash
redis> JSON.SET doc $ '{"data": {"name": "Alice", "scores": [1, 2, 3]}}'
redis> JSON.JMESPATH doc "json_encode(data)"
"\"{\\\"name\\\":\\\"Alice\\\",\\\"scores\\\":[1,2,3]}\""

redis> JSON.SET doc $ '{"arr": [1, 2, 3]}'
redis> JSON.JMESPATH doc "json_encode(arr)"
"\"[1,2,3]\""
```

#### `json_decode(string) -> any`
Parse a JSON string into a value. Returns null for invalid JSON.

```bash
redis> JSON.SET doc $ '{"json_str": "{\"name\": \"Bob\", \"age\": 30}"}'
redis> JSON.JMESPATH doc "json_decode(json_str)"
"{\"age\":30,\"name\":\"Bob\"}"

redis> JSON.JMESPATH doc "json_decode(json_str).name"
"\"Bob\""

# Invalid JSON returns null
redis> JSON.SET doc $ '{"json_str": "not valid json"}'
redis> JSON.JMESPATH doc "json_decode(json_str)"
"null"
```

### String Case Aliases (3)

These are snake_case aliases for consistency with other functions.

#### `upper_case(string) -> string`
Alias for `upper()`. Convert string to uppercase.

```bash
redis> JSON.SET doc $ '{"name": "hello"}'
redis> JSON.JMESPATH doc "upper_case(name)"
"\"HELLO\""
```

#### `lower_case(string) -> string`
Alias for `lower()`. Convert string to lowercase.

```bash
redis> JSON.SET doc $ '{"name": "HELLO"}'
redis> JSON.JMESPATH doc "lower_case(name)"
"\"hello\""
```

#### `title_case(string) -> string`
Alias for `title()`. Capitalize the first letter of each word.

```bash
redis> JSON.SET doc $ '{"name": "hello world"}'
redis> JSON.JMESPATH doc "title_case(name)"
"\"Hello World\""
```

#### `camel_case(string) -> string`
Convert string to camelCase.

```bash
redis> JSON.SET doc $ '{"s": "hello_world"}'
redis> JSON.JMESPATH doc "camel_case(s)"
"\"helloWorld\""

redis> JSON.SET doc $ '{"s": "Hello World"}'
redis> JSON.JMESPATH doc "camel_case(s)"
"\"helloWorld\""
```

#### `snake_case(string) -> string`
Convert string to snake_case.

```bash
redis> JSON.SET doc $ '{"s": "helloWorld"}'
redis> JSON.JMESPATH doc "snake_case(s)"
"\"hello_world\""

redis> JSON.SET doc $ '{"s": "Hello World"}'
redis> JSON.JMESPATH doc "snake_case(s)"
"\"hello_world\""
```

#### `kebab_case(string) -> string`
Convert string to kebab-case.

```bash
redis> JSON.SET doc $ '{"s": "helloWorld"}'
redis> JSON.JMESPATH doc "kebab_case(s)"
"\"hello-world\""

redis> JSON.SET doc $ '{"s": "Hello World"}'
redis> JSON.JMESPATH doc "kebab_case(s)"
"\"hello-world\""
```

#### `url_encode(string) -> string`
URL-encode a string.

```bash
redis> JSON.SET doc $ '{"q": "hello world"}'
redis> JSON.JMESPATH doc "url_encode(q)"
"\"hello%20world\""

redis> JSON.SET doc $ '{"q": "foo=bar&baz=qux"}'
redis> JSON.JMESPATH doc "url_encode(q)"
"\"foo%3Dbar%26baz%3Dqux\""
```

#### `url_decode(string) -> string`
URL-decode a string.

```bash
redis> JSON.SET doc $ '{"q": "hello%20world"}'
redis> JSON.JMESPATH doc "url_decode(q)"
"\"hello world\""
```

### Statistics Functions (3)

#### `mode(array) -> any`
Return the most frequently occurring value in an array.

```bash
redis> JSON.SET doc $ '{"scores": [1, 2, 2, 3, 2, 4]}'
redis> JSON.JMESPATH doc "mode(scores)"
"2"

redis> JSON.SET doc $ '{"tags": ["a", "b", "a", "c", "a"]}'
redis> JSON.JMESPATH doc "mode(tags)"
"\"a\""
```

#### `variance(array) -> number`
Calculate the population variance of a numeric array.

```bash
redis> JSON.SET doc $ '{"data": [2, 4, 4, 4, 5, 5, 7, 9]}'
redis> JSON.JMESPATH doc "variance(data)"
"4.0"
```

#### `stddev(array) -> number`
Calculate the population standard deviation of a numeric array.

```bash
redis> JSON.SET doc $ '{"data": [2, 4, 4, 4, 5, 5, 7, 9]}'
redis> JSON.JMESPATH doc "stddev(data)"
"2.0"
```

### Path/URL Functions (5)

#### `path_basename(string) -> string`
Extract the filename from a path.

```bash
redis> JSON.SET doc $ '{"p": "/home/user/file.txt"}'
redis> JSON.JMESPATH doc "path_basename(p)"
"\"file.txt\""
```

#### `path_dirname(string) -> string`
Extract the directory from a path.

```bash
redis> JSON.SET doc $ '{"p": "/home/user/file.txt"}'
redis> JSON.JMESPATH doc "path_dirname(p)"
"\"/home/user\""
```

#### `path_ext(string) -> string`
Extract the file extension from a path (including the dot).

```bash
redis> JSON.SET doc $ '{"p": "/home/user/file.txt"}'
redis> JSON.JMESPATH doc "path_ext(p)"
"\".txt\""

redis> JSON.SET doc $ '{"p": "Makefile"}'
redis> JSON.JMESPATH doc "path_ext(p)"
"\"\""
```

#### `path_join(array, separator?) -> string`
Join path segments with a separator. Separator defaults to "/".

```bash
redis> JSON.SET doc $ '{"parts": ["home", "user", "docs", "file.txt"]}'
redis> JSON.JMESPATH doc "path_join(parts)"
"\"home/user/docs/file.txt\""

redis> JSON.JMESPATH doc "path_join(parts, '\\\\')"
"\"home\\\\user\\\\docs\\\\file.txt\""

redis> JSON.SET doc $ '{"dir": "var", "subdir": "log", "file": "app.log"}'
redis> JSON.JMESPATH doc "path_join([dir, subdir, file])"
"\"var/log/app.log\""
```

#### `url_parse(string) -> object`
Parse a URL into its components. Returns an object with: `scheme`, `host`, `port`, `path`, `query`, `fragment`, `username`, `password`, and `origin`. Returns null for invalid URLs.

```bash
redis> JSON.SET doc $ '{"url": "https://user:pass@example.com:8080/path?query=1#section"}'
redis> JSON.JMESPATH doc "url_parse(url)"
"{\"scheme\":\"https\",\"host\":\"example.com\",\"port\":8080,\"path\":\"/path\",\"query\":\"query=1\",\"fragment\":\"section\",\"username\":\"user\",\"password\":\"pass\",\"origin\":\"https://example.com:8080\"}"

# Access specific components
redis> JSON.JMESPATH doc "url_parse(url).host"
"\"example.com\""

redis> JSON.JMESPATH doc "url_parse(url).port"
"8080"

# Simple URL (optional components are null)
redis> JSON.SET doc $ '{"url": "https://example.com/path"}'
redis> JSON.JMESPATH doc "url_parse(url).port"
"null"

redis> JSON.JMESPATH doc "url_parse(url).origin"
"\"https://example.com\""

# Invalid URL returns null
redis> JSON.SET doc $ '{"url": "not a url"}'
redis> JSON.JMESPATH doc "url_parse(url)"
"null"
```

### Validation Functions (8)

#### `is_email(string) -> boolean`
Check if string is a valid email address format.

```bash
redis> JSON.SET doc $ '{"e": "user@example.com"}'
redis> JSON.JMESPATH doc "is_email(e)"
"true"

redis> JSON.SET doc $ '{"e": "not-an-email"}'
redis> JSON.JMESPATH doc "is_email(e)"
"false"
```

#### `is_url(string) -> boolean`
Check if string is a valid HTTP/HTTPS URL format.

```bash
redis> JSON.SET doc $ '{"u": "https://example.com/path?query=1"}'
redis> JSON.JMESPATH doc "is_url(u)"
"true"

redis> JSON.SET doc $ '{"u": "not-a-url"}'
redis> JSON.JMESPATH doc "is_url(u)"
"false"
```

#### `is_uuid(string) -> boolean`
Check if string is a valid UUID format.

```bash
redis> JSON.SET doc $ '{"id": "550e8400-e29b-41d4-a716-446655440000"}'
redis> JSON.JMESPATH doc "is_uuid(id)"
"true"
```

#### `is_ipv4(string) -> boolean`
Check if string is a valid IPv4 address.

```bash
redis> JSON.SET doc $ '{"ip": "192.168.1.1"}'
redis> JSON.JMESPATH doc "is_ipv4(ip)"
"true"

redis> JSON.SET doc $ '{"ip": "256.1.1.1"}'
redis> JSON.JMESPATH doc "is_ipv4(ip)"
"false"
```

#### `is_ipv6(string) -> boolean`
Check if string is a valid IPv6 address.

```bash
redis> JSON.SET doc $ '{"ip": "::1"}'
redis> JSON.JMESPATH doc "is_ipv6(ip)"
"true"

redis> JSON.SET doc $ '{"ip": "2001:0db8:85a3:0000:0000:8a2e:0370:7334"}'
redis> JSON.JMESPATH doc "is_ipv6(ip)"
"true"
```

#### `is_empty(any) -> boolean`
Check if a value is empty. Returns true for: empty strings, empty arrays, empty objects, and null. Numbers and booleans return false.

```bash
redis> JSON.SET doc $ '{"s": "", "arr": [], "obj": {}, "n": null}'
redis> JSON.JMESPATH doc "is_empty(s)"
"true"

redis> JSON.JMESPATH doc "is_empty(arr)"
"true"

redis> JSON.JMESPATH doc "is_empty(obj)"
"true"

redis> JSON.SET doc $ '{"s": "hello", "arr": [1, 2]}'
redis> JSON.JMESPATH doc "is_empty(s)"
"false"
```

#### `is_blank(string) -> boolean`
Check if a string is empty or contains only whitespace. Returns null for non-strings.

```bash
redis> JSON.SET doc $ '{"s": "   "}'
redis> JSON.JMESPATH doc "is_blank(s)"
"true"

redis> JSON.SET doc $ '{"s": ""}'
redis> JSON.JMESPATH doc "is_blank(s)"
"true"

redis> JSON.SET doc $ '{"s": "  hello  "}'
redis> JSON.JMESPATH doc "is_blank(s)"
"false"
```

#### `is_json(string) -> boolean`
Check if a string contains valid JSON. Returns null for non-strings.

```bash
redis> JSON.SET doc $ '{"s": "{\"a\": 1}"}'
redis> JSON.JMESPATH doc "is_json(s)"
"true"

redis> JSON.SET doc $ '{"s": "[1, 2, 3]"}'
redis> JSON.JMESPATH doc "is_json(s)"
"true"

redis> JSON.SET doc $ '{"s": "{not json}"}'
redis> JSON.JMESPATH doc "is_json(s)"
"false"
```

### Utility/Conditional Functions (7)

#### `now(fallback?) -> number`
Returns current Unix timestamp in seconds. If a fallback value is provided, returns that instead (for deterministic behavior in replication scenarios).

```bash
redis> JSON.SET doc $ '{"expires": 1700000000}'
redis> JSON.JMESPATH doc "expires > now()"
"true"  # or "false" depending on current time

# With fallback for deterministic behavior
redis> JSON.JMESPATH doc "now(`1700000000`)"
"1700000000"
```

**Note:** Without a fallback, this function is non-deterministic and may produce different values across replicas. Use a fallback when determinism is required.

#### `now_ms(fallback?) -> number`
Returns current Unix timestamp in milliseconds. If a fallback value is provided, returns that instead (for deterministic behavior in replication scenarios).

```bash
redis> JSON.SET doc $ '{}'
redis> JSON.JMESPATH doc "now_ms()"
"1733417234567"  # example output

# With fallback for deterministic behavior
redis> JSON.JMESPATH doc "now_ms(`1733417234567`)"
"1733417234567"
```

**Note:** Without a fallback, this function is non-deterministic and may produce different values across replicas. Use a fallback when determinism is required.

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

#### `random(min?, max?) -> number`
Generate a random floating-point number. With no arguments, returns a value in [0, 1). With two arguments, returns a value in [min, max).

**Note:** This function is non-deterministic and may produce different values across replicas.

```bash
redis> JSON.SET doc $ '{}'
redis> JSON.JMESPATH doc "random()"
"0.7234..."  # random value in [0, 1)

redis> JSON.JMESPATH doc "random(`1`, `10`)"
"5.234..."  # random value in [1, 10)
```

#### `uuid() -> string`
Generate a random UUID v4 string.

**Note:** This function is non-deterministic and may produce different values across replicas.

```bash
redis> JSON.SET doc $ '{}'
redis> JSON.JMESPATH doc "uuid()"
"\"550e8400-e29b-41d4-a716-446655440000\""
```

#### `coalesce(...args) -> any`
Return the first non-null argument. Variadic function that accepts any number of arguments.

```bash
redis> JSON.SET doc $ '{"a": null, "b": null, "c": "found"}'
redis> JSON.JMESPATH doc "coalesce(a, b, c)"
"\"found\""

redis> JSON.SET doc $ '{"primary": null, "secondary": "backup", "default": "fallback"}'
redis> JSON.JMESPATH doc "coalesce(primary, secondary, default)"
"\"backup\""

# All null returns null
redis> JSON.SET doc $ '{"a": null, "b": null}'
redis> JSON.JMESPATH doc "coalesce(a, b)"
"null"

# With literal fallback
redis> JSON.SET doc $ '{"config": null}'
redis> JSON.JMESPATH doc "coalesce(config, `{\"timeout\": 30}`)"
"{\"timeout\":30}"
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

## Non-Deterministic Functions and Replication

Some functions produce non-deterministic results, which can cause issues in Redis replication scenarios where the same command should produce identical results on primary and replica nodes.

### Functions with Deterministic Fallbacks

These functions are non-deterministic by default but accept an optional parameter for deterministic behavior:

| Function | Non-deterministic | Deterministic |
|----------|-------------------|---------------|
| `now()` | `now()` | `now(\`1700000000\`)` |
| `now_ms()` | `now_ms()` | `now_ms(\`1700000000000\`)` |
| `shuffle(arr)` | `shuffle(arr)` | `shuffle(arr, \`42\`)` (seed) |
| `sample(arr, n)` | `sample(arr, n)` | `sample(arr, n, \`42\`)` (seed) |

### Always Non-Deterministic

These functions always produce non-deterministic results:

| Function | Notes |
|----------|-------|
| `random()` | Returns random float |
| `uuid()` | Generates random UUID |

**Important:** Since `JSON.JMESPATH` is a read-only command (it doesn't modify data), the primary concern is if your application logic depends on the result being identical across replicas. For most use cases where you're simply querying a replica, the non-determinism is acceptable.

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

### Custom String Functions (27)
`lower`, `upper`, `trim`, `capitalize`, `title`, `split`, `replace`, `repeat`, `pad_left`, `pad_right`, `substr`, `slice`, `index_of`, `last_index_of`, `concat`, `upper_case`, `lower_case`, `title_case`, `camel_case`, `snake_case`, `kebab_case`, `url_encode`, `url_decode`, `truncate`, `trim_start`, `trim_end`, `regex_match`, `regex_extract`, `regex_replace`, `wrap`, `format`

### Custom Array Functions (26)
`unique`, `zip`, `chunk`, `take`, `drop`, `flatten_deep`, `compact`, `range`, `index_at`, `includes`, `find_index`, `first`, `last`, `difference`, `intersection`, `union`, `group_by`, `frequencies`, `mode`, `nth`, `interleave`, `rotate`, `partition`, `shuffle`, `sample`, `cartesian`

### Custom Object Functions (9)
`entries`, `from_entries`, `pick`, `omit`, `deep_merge`, `invert`, `rename_keys`, `flatten_keys`, `unflatten_keys`

### Custom Math/Statistics Functions (23)
`round`, `floor_fn`, `ceil_fn`, `abs_fn`, `mod_fn`, `pow`, `sqrt`, `log`, `clamp`, `median`, `percentile`, `variance`, `stddev`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sign`, `atan2`, `deg_to_rad`, `rad_to_deg`

### Custom Type Functions (10)
`to_string`, `to_number`, `to_boolean`, `type_of`, `is_string`, `is_number`, `is_boolean`, `is_array`, `is_object`, `is_null`

### Custom Utility/Conditional Functions (7)
`now`, `now_ms`, `default`, `if`, `random`, `uuid`, `coalesce`

### Custom Hash/Checksum Functions (4)
`md5`, `sha1`, `sha256`, `crc32`

### Custom Encoding Functions (8)
`base64_encode`, `base64_decode`, `url_encode`, `url_decode`, `hex_encode`, `hex_decode`, `json_encode`, `json_decode`

### Custom Path/URL Functions (5)
`path_basename`, `path_dirname`, `path_ext`, `path_join`, `url_parse`

### Custom Validation Functions (8)
`is_email`, `is_url`, `is_uuid`, `is_ipv4`, `is_ipv6`, `is_empty`, `is_blank`, `is_json`

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

### Array Functions
| Function | Description | Example |
|----------|-------------|---------|
| `index_by(arr, &expr)` | Create lookup object | `index_by(users, &id)` |

### Object Functions
| Function | Description | Example |
|----------|-------------|---------|
| `map_values(obj, &expr)` | Transform all values | `map_values(prices, &round(@, 2))` |
| `map_keys(obj, &expr)` | Transform all keys | `map_keys(obj, &lower(@))` |
| `filter_keys(obj, &expr)` | Filter by key | `filter_keys(obj, &starts_with(@, 'user_'))` |
| `filter_values(obj, &expr)` | Filter by value | `filter_values(obj, &@ != null)` |

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

### Conditional/Logic Functions
| Function | Description | Example |
|----------|-------------|---------|
| `switch(val, cases, default)` | Switch/case | `switch(status, {1: 'ok', 2: 'err'}, 'unknown')` |
| `all(arr, &expr)` | All match predicate | `all(items, &@ > 0)` |
| `any(arr, &expr)` | Any matches predicate | `any(items, &@ > 100)` |
| `none(arr, &expr)` | None match predicate | `none(items, &is_null(@))` |

### Implementation Notes

Some functions face technical limitations:

- **Expression-based functions** (`index_by`, `all`, `any`, `map_values`, etc.): Require access to the JMESPath interpreter, which is not publicly exposed by the `jmespath` crate. Would require forking the crate or upstream changes.

- **Date functions**: Would benefit from the `chrono` crate for robust parsing/formatting.

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
