# JMESPath for RedisJSON - Use Cases & Value Proposition

## Why JMESPath over JSONPath?

JSONPath is great for *addressing* data. JMESPath is great for *transforming* data. The difference matters when you want to:

1. **Reshape output** - Return data in a different structure than stored
2. **Compute values** - Aggregations, calculations, string manipulation
3. **Chain operations** - Filter → Sort → Project → Aggregate in one query
4. **Reduce payload** - Return exactly what you need, nothing more

## Core Advantages

### 1. Data Reshaping with Multi-Select Hashes

**JSONPath:** `$.users[*].[name, status]`  
**Result:** `[["Alice", "active"], ["Bob", "inactive"]]` (unstructured)

**JMESPath:** `users[*].{user_name: name, current_status: status}`  
**Result:** `[{"user_name": "Alice", "current_status": "active"}, ...]` (structured)

### 2. Built-in Functions & Aggregation

**JSONPath:** `$.products[*].price` → `[1200, 25, 75]` (client must sum)

**JMESPath:** `max(products[*].price)` → `1200` (computed server-side)

### 3. Piping for Chained Operations

**JMESPath:** `log_entries[?level == 'ERROR'] | [0].msg`

Filter → Select first → Extract field, all in one expression.

---

## AI/ML Workflow Use Cases

### Feature Extraction for ML Pipelines

Pull specific features into a vector for embedding or model input:

```json
{
  "user_id": "u123",
  "demographics": {"age": 34, "region": "US-West"},
  "behavior": {
    "sessions_last_30d": 47,
    "avg_session_minutes": 12.3,
    "purchase_count": 8
  }
}
```

**JMESPath:**
```
{user: user_id, features: [demographics.age, behavior.sessions_last_30d, behavior.avg_session_minutes, behavior.purchase_count]}
```

**Result:** `{"user": "u123", "features": [34, 47, 12.3, 8]}`

Clean numeric array ready for model input, with ID attached for correlation.

---

### RAG Context Assembly

Pull chunked documents filtered by token budget for LLM prompts:

```json
{
  "chunks": [
    {"idx": 0, "text": "Redis is an in-memory data store...", "tokens": 45},
    {"idx": 1, "text": "It supports various data structures...", "tokens": 38},
    {"idx": 2, "text": "Common use cases include caching...", "tokens": 52}
  ]
}
```

**JMESPath:**
```
chunks[?tokens < `50`] | [*].text | join('\n\n', @)
```

**Result:** Concatenated string filtered by token budget, ready for prompt injection.

---

### Embedding Batch Preparation

Normalize product descriptions for vectorization:

```json
{
  "catalog": [
    {"sku": "A1", "title": "Wireless Mouse", "desc": "Ergonomic design...", "category": "electronics"},
    {"sku": "A2", "title": "USB Cable", "desc": null, "category": "electronics"},
    {"sku": "A3", "title": "Notebook", "desc": "200 pages lined...", "category": "office"}
  ]
}
```

**JMESPath:**
```
catalog[?desc != null].{id: sku, text: join(' - ', [title, desc]), meta: {cat: category}}
```

Filters nulls, concatenates fields, restructures metadata—one query.

---

### Aggregating Similarity Search Results

After vector search returns IDs, compute summary stats:

```json
{
  "results": [
    {"id": "p1", "score": 0.92, "price": 299, "rating": 4.5},
    {"id": "p2", "score": 0.87, "price": 349, "rating": 4.8},
    {"id": "p3", "score": 0.85, "price": 275, "rating": 4.2}
  ]
}
```

**JMESPath:**
```
{avg_score: avg(results[*].score), price_range: [min(results[*].price), max(results[*].price)], top_match: results[0].id}
```

**Result:** `{"avg_score": 0.88, "price_range": [275, 349], "top_match": "p1"}`

---

### Agent Tool Response Formatting

LLM agents need responses in predictable shapes:

```json
{
  "session": "s789",
  "turns": [
    {"role": "user", "content": "What's my balance?", "ts": 1700000001},
    {"role": "assistant", "content": "Your balance is $142.50", "ts": 1700000002},
    {"role": "user", "content": "Transfer $50 to savings", "ts": 1700000003}
  ],
  "context": {"account_id": "acct_123", "verified": true}
}
```

**JMESPath:**
```
{recent: turns[-2:], account: context.account_id, is_verified: context.verified}
```

Agent gets exactly the context window it needs, pre-shaped.

---

## Extended Functions Use Cases (jmespath_extensions)

With 189+ extension functions, additional capabilities unlock:

### Fuzzy Search Post-Processing

Re-rank vector search results by string similarity:

```
results[?jaro_winkler(title, 'wireless mouse') > `0.8`] | sort_by(@, &jaro_winkler(title, 'wireless mouse')) | reverse(@)
```

### Geo-Aware Retrieval

Sort stores by proximity:

```
stores[*].{id: id, name: name, distance_km: haversine_km(lat, lng, `37.76`, `-122.45`)} | sort_by(@, &distance_km) | [:3]
```

### Datetime Filtering

Filter events without client-side parsing:

```
events[?parse_date(ts, '%Y-%m-%dT%H:%M:%SZ') > parse_date('2024-01-15T12:00:00Z', '%Y-%m-%dT%H:%M:%SZ')].type
```

### Data Validation at Query Time

Only return contacts with valid emails:

```
contacts[?is_email(email)].{name: name, email: email}
```

### Phonetic Matching

Find contacts despite misspellings:

```
contacts[?sounds_like(name, 'Jon')].name
```

Returns "John", "Jon", "Jonathan" etc.

### Network/CIDR Filtering

Filter hosts by IP range:

```
hosts[?cidr_contains('10.0.0.0/16', ip)].name
```

---

## MCP Server Integration

An MCP server exposing RedisJSON + JMESPath as tools for LLM agents:

```typescript
tools: [
  {
    name: "redis_json_query",
    description: "Query JSON with extended JMESPath (189+ functions)",
    parameters: {
      key: "string",
      expr: "string - JMESPath expression"
    }
  }
]
```

**Example agent flow:**

```
Agent: "What are the user's recent orders over $100?"

Tool call: redis_json_query(
  key: "user:123",
  expr: "orders[?total > `100`] | sort_by(@, &date)[-3:].{id: order_id, amount: total}"
)

Tool response: [
  {"id": "ord_a", "amount": 150},
  {"id": "ord_b", "amount": 220}
]
```

The agent never sees full order history—just the filtered, sorted, projected slice.

---

## The Pitch

> "JMESPath transforms your Redis JSON queries from 'get me this data' to 'get me this answer.' 
> Filter, sort, reshape, aggregate—all at the data layer. 
> Fewer round trips, smaller payloads, cleaner application code.
> For AI/ML workloads: agents get exactly the data shape they need, 
> computed server-side, with 189 transformation functions built in."
