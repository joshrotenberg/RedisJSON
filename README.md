[![GitHub issues](https://img.shields.io/github/release/RedisJSON/RedisJSON.svg)](https://github.com/RedisJSON/RedisJSON/releases/latest)
[![CircleCI](https://circleci.com/gh/RedisJSON/RedisJSON/tree/master.svg?style=svg)](https://circleci.com/gh/RedisJSON/RedisJSON/tree/master)
[![macos](https://github.com/RedisJSON/RedisJSON/workflows/macos/badge.svg)](https://github.com/RedisJSON/RedisJSON/actions?query=workflow%3Amacos)
[![Dockerhub](https://img.shields.io/docker/pulls/redis/redis-stack-server?label=redis-stack-server)](https://hub.docker.com/r/redis/redis-stack-server/)
[![Codecov](https://codecov.io/gh/RedisJSON/RedisJSON/branch/master/graph/badge.svg)](https://codecov.io/gh/RedisJSON/RedisJSON)

# RedisJSON

[![Discord](https://img.shields.io/discord/697882427875393627?style=flat-square)](https://discord.gg/QUkjSsk)

<img src="docs/docs/images/logo.svg" alt="logo" width="300"/>

> [!NOTE]
> Starting with Redis 8, the JSON data structure is integral to Redis. You don't need to install this module separately.
>
> We no longer release standalone versions of RedisJSON.
>
> See https://github.com/redis/redis

> [!NOTE]
> 32 bit systems are not supported.

## Overview

RedisJSON is a [Redis](https://redis.io/) module that implements [ECMA-404 The JSON Data Interchange Standard](https://json.org/) as a native data type. It allows storing, updating, and fetching JSON values from Redis keys (documents).

## Primary features

* Full support of the JSON standard
* [JSONPath](https://goessner.net/articles/JsonPath/) syntax for selecting elements inside documents
* [JMESPath](https://jmespath.org/) query support for powerful data extraction and transformation (see below)
* Documents are stored as binary data in a tree structure, allowing fast access to sub-elements
* Typed atomic operations for all JSON value types
* Secondary index support when combined with [RediSearch](https://redis.io/docs/latest/develop/interact/search-and-query/)

## JMESPath Support

This fork adds experimental JMESPath query support via the `JSON.JMESPATH` command. JMESPath is a query language for JSON that complements JSONPath with powerful features like:

* **Projections**: Extract and reshape data with `[*].field` syntax
* **Filters**: Query with conditions like `items[?price > \`100\`]`
* **Pipes**: Chain operations with `expression | sort(@) | [0]`
* **Multiselect**: Reshape output with `{name: field1, value: field2}`
* **175+ functions**: 26 standard JMESPath + 150+ custom functions across 25 categories
* **Configurable**: Enable/disable function categories via module args
* **Expression caching**: Thread-local LRU cache (256 entries) for high-throughput workloads
* **RESP3 support**: `FORMAT EXPAND` returns native Redis types instead of JSON strings

```bash
# Basic usage
redis> JSON.SET users $ '[{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]'
OK
redis> JSON.JMESPATH users "[*].name"
["Alice","Bob"]
redis> JSON.JMESPATH users "[?age > `25`].name"
["Alice"]

# Reshape + aggregate
redis> JSON.JMESPATH users "{names: [*].name, avg_age: avg([*].age)}"
{"names":["Alice","Bob"],"avg_age":27.5}

# Custom functions
redis> JSON.JMESPATH users "[*].{name: upper(name), adult: age >= `18`}"
[{"name":"ALICE","adult":true},{"name":"BOB","adult":true}]

redis> JSON.JMESPATH users "unique([*].age) | sort(@)"
[25,30]
```

**25 function categories** including string, array, object, math, type, utility, hash, encoding, url, regex, random, validation, path, datetime, fuzzy, phonetic, expression, geo, semver, network, ids, text, duration, color, and computing.

See [docs/jmespath.md](docs/jmespath.md) for full documentation including configuration options, or [jmespath.org](https://jmespath.org/) for the language specification.

## Documentation

Read the docs at <https://redis.io/docs/latest/develop/data-types/json/>

## License

Starting with Redis 8, RedisJSON is licensed under your choice of: (i) Redis Source Available License 2.0 (RSALv2); (ii) the Server Side Public License v1 (SSPLv1); or (iii) the GNU Affero General Public License version 3 (AGPLv3). Please review the license folder for the full license terms and conditions. Prior versions remain subject to (i) and (ii).

## Code contributions

By contributing code to this Redis module in any form, including sending a pull request via GitHub, a code fragment or patch via private email or public discussion groups, you agree to release your code under the terms of the Redis Software Grant and Contributor License Agreement. Please see the CONTRIBUTING.md file in this source distribution for more information. For security bugs and vulnerabilities, please see SECURITY.md. 
