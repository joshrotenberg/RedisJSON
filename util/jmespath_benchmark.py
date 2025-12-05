#!/usr/bin/env python3
"""
Benchmark script for JMESPath expression caching in RedisJSON.

This script measures the performance benefits of expression caching by:
1. Running repeated queries with the same expression (cache hits)
2. Running queries with different expressions (cache misses)
3. Comparing JMESPath vs JSONPath for common operations
"""

import argparse
import json
import statistics
import time
from typing import Callable

import redis


def benchmark(
    func: Callable,
    iterations: int = 1000,
    warmup: int = 100,
) -> dict:
    """Run a benchmark and return timing statistics."""
    # Warmup
    for _ in range(warmup):
        func()

    # Actual benchmark
    times = []
    for _ in range(iterations):
        start = time.perf_counter()
        func()
        elapsed = time.perf_counter() - start
        times.append(elapsed * 1000)  # Convert to ms

    return {
        "min_ms": min(times),
        "max_ms": max(times),
        "mean_ms": statistics.mean(times),
        "median_ms": statistics.median(times),
        "stdev_ms": statistics.stdev(times) if len(times) > 1 else 0,
        "p99_ms": sorted(times)[int(len(times) * 0.99)],
        "ops_per_sec": 1000 / statistics.mean(times),
    }


def print_result(name: str, result: dict):
    """Print benchmark results in a formatted way."""
    print(f"\n{name}")
    print("-" * len(name))
    print(f"  Mean:      {result['mean_ms']:.4f} ms")
    print(f"  Median:    {result['median_ms']:.4f} ms")
    print(f"  Min:       {result['min_ms']:.4f} ms")
    print(f"  Max:       {result['max_ms']:.4f} ms")
    print(f"  Stdev:     {result['stdev_ms']:.4f} ms")
    print(f"  P99:       {result['p99_ms']:.4f} ms")
    print(f"  Ops/sec:   {result['ops_per_sec']:.0f}")


def setup_test_data(r: redis.Redis):
    """Set up test data for benchmarks."""
    # Small document
    small_doc = {"name": "Alice", "age": 30, "city": "NYC"}
    r.execute_command("JSON.SET", "small", "$", json.dumps(small_doc))

    # Medium document - array of users
    users = [
        {
            "name": f"User{i}",
            "age": 20 + (i % 50),
            "active": i % 2 == 0,
            "score": i * 1.5,
        }
        for i in range(100)
    ]
    r.execute_command("JSON.SET", "users", "$", json.dumps(users))

    # Large document - nested e-commerce data
    products = []
    for i in range(50):
        products.append(
            {
                "id": f"PROD{i:04d}",
                "name": f"Product {i}",
                "price": 10.0 + (i * 2.5),
                "category": ["electronics", "clothing", "home", "sports"][i % 4],
                "tags": [f"tag{j}" for j in range(i % 5 + 1)],
                "reviews": [
                    {"user": f"user{j}", "rating": 3 + (j % 3), "text": f"Review {j}"}
                    for j in range(i % 10)
                ],
                "in_stock": i % 3 != 0,
            }
        )
    large_doc = {
        "store": "Test Store",
        "products": products,
        "metadata": {"total": len(products), "last_updated": "2024-01-15"},
    }
    r.execute_command("JSON.SET", "store", "$", json.dumps(large_doc))


def run_cache_hit_benchmark(r: redis.Redis, iterations: int):
    """Benchmark repeated queries with the same expression (cache hits)."""
    print("\n" + "=" * 60)
    print("CACHE HIT BENCHMARK (Same expression repeated)")
    print("=" * 60)

    # Simple field access - cache hit
    result = benchmark(
        lambda: r.execute_command("JSON.JMESPATH", "small", "name"),
        iterations=iterations,
    )
    print_result("Simple field access (small doc)", result)

    # Array projection - cache hit
    result = benchmark(
        lambda: r.execute_command("JSON.JMESPATH", "users", "[*].name"),
        iterations=iterations,
    )
    print_result("Array projection [*].name (100 users)", result)

    # Filter expression - cache hit
    result = benchmark(
        lambda: r.execute_command("JSON.JMESPATH", "users", "[?active].name"),
        iterations=iterations,
    )
    print_result("Filter [?active].name (100 users)", result)

    # Complex expression - cache hit
    result = benchmark(
        lambda: r.execute_command(
            "JSON.JMESPATH",
            "store",
            "products[?in_stock && price < `50`] | sort_by(@, &price) | [0:5].{name: name, price: price}",
        ),
        iterations=iterations,
    )
    print_result("Complex query with filter+sort+slice (50 products)", result)


def run_cache_miss_benchmark(r: redis.Redis, iterations: int):
    """Benchmark queries with varying expressions (cache misses)."""
    print("\n" + "=" * 60)
    print("CACHE MISS BENCHMARK (Different expressions)")
    print("=" * 60)

    # Vary the filter value each time
    counter = [0]

    def varying_filter():
        counter[0] += 1
        age = 20 + (counter[0] % 50)
        r.execute_command("JSON.JMESPATH", "users", f"[?age == `{age}`].name")

    result = benchmark(varying_filter, iterations=iterations)
    print_result("Varying filter value (cache miss)", result)

    # Compare with fixed expression
    result = benchmark(
        lambda: r.execute_command("JSON.JMESPATH", "users", "[?age == `30`].name"),
        iterations=iterations,
    )
    print_result("Fixed filter value (cache hit)", result)


def run_custom_function_benchmark(r: redis.Redis, iterations: int):
    """Benchmark custom JMESPath functions."""
    print("\n" + "=" * 60)
    print("CUSTOM FUNCTION BENCHMARK")
    print("=" * 60)

    # String functions
    result = benchmark(
        lambda: r.execute_command(
            "JSON.JMESPATH", "users", "[*].{upper: upper(name), lower: lower(name)}"
        ),
        iterations=iterations,
    )
    print_result("String functions upper/lower (100 users)", result)

    # Math functions
    result = benchmark(
        lambda: r.execute_command(
            "JSON.JMESPATH",
            "users",
            "[*].{rounded: round(score), clamped: clamp(age, `25`, `40`)}",
        ),
        iterations=iterations,
    )
    print_result("Math functions round/clamp (100 users)", result)

    # Array functions
    result = benchmark(
        lambda: r.execute_command(
            "JSON.JMESPATH", "store", "products[*].category | unique(@)"
        ),
        iterations=iterations,
    )
    print_result("Array function unique (50 products)", result)


def run_jsonpath_comparison(r: redis.Redis, iterations: int):
    """Compare JMESPath vs JSONPath for equivalent operations."""
    print("\n" + "=" * 60)
    print("JMESPATH vs JSONPATH COMPARISON")
    print("=" * 60)

    # Simple field access
    jmespath_result = benchmark(
        lambda: r.execute_command("JSON.JMESPATH", "small", "name"),
        iterations=iterations,
    )
    jsonpath_result = benchmark(
        lambda: r.execute_command("JSON.GET", "small", "$.name"),
        iterations=iterations,
    )
    print_result("JMESPath: simple field", jmespath_result)
    print_result("JSONPath: simple field", jsonpath_result)
    ratio = jmespath_result["mean_ms"] / jsonpath_result["mean_ms"]
    print(f"  Ratio (JMESPath/JSONPath): {ratio:.2f}x")

    # Array projection
    jmespath_result = benchmark(
        lambda: r.execute_command("JSON.JMESPATH", "users", "[*].name"),
        iterations=iterations,
    )
    jsonpath_result = benchmark(
        lambda: r.execute_command("JSON.GET", "users", "$[*].name"),
        iterations=iterations,
    )
    print_result("JMESPath: array projection", jmespath_result)
    print_result("JSONPath: array projection", jsonpath_result)
    ratio = jmespath_result["mean_ms"] / jsonpath_result["mean_ms"]
    print(f"  Ratio (JMESPath/JSONPath): {ratio:.2f}x")

    # Filter
    jmespath_result = benchmark(
        lambda: r.execute_command("JSON.JMESPATH", "users", "[?age > `30`].name"),
        iterations=iterations,
    )
    jsonpath_result = benchmark(
        lambda: r.execute_command("JSON.GET", "users", "$[?(@.age > 30)].name"),
        iterations=iterations,
    )
    print_result("JMESPath: filter age > 30", jmespath_result)
    print_result("JSONPath: filter age > 30", jsonpath_result)
    ratio = jmespath_result["mean_ms"] / jsonpath_result["mean_ms"]
    print(f"  Ratio (JMESPath/JSONPath): {ratio:.2f}x")


def run_transformation_benchmark(r: redis.Redis, iterations: int):
    """Benchmark data transformation capabilities unique to JMESPath."""
    print("\n" + "=" * 60)
    print("DATA TRANSFORMATION BENCHMARK (JMESPath unique)")
    print("=" * 60)

    # Multi-select hash (reshape data)
    result = benchmark(
        lambda: r.execute_command(
            "JSON.JMESPATH",
            "users",
            "[*].{userName: name, userAge: age, isActive: active}",
        ),
        iterations=iterations,
    )
    print_result("Reshape with multi-select hash (100 users)", result)

    # Pipe with sort
    result = benchmark(
        lambda: r.execute_command(
            "JSON.JMESPATH",
            "users",
            "[*] | sort_by(@, &age) | reverse(@) | [0:10].name",
        ),
        iterations=iterations,
    )
    print_result("Pipe: sort, reverse, slice, project (100 users)", result)

    # Aggregation
    result = benchmark(
        lambda: r.execute_command(
            "JSON.JMESPATH",
            "users",
            "{total: length(@), avgAge: avg([*].age), maxScore: max([*].score)}",
        ),
        iterations=iterations,
    )
    print_result("Aggregation: length, avg, max (100 users)", result)


def main():
    parser = argparse.ArgumentParser(
        description="Benchmark JMESPath expression caching in RedisJSON"
    )
    parser.add_argument(
        "-p", "--port", type=int, default=6379, help="Redis port (default: 6379)"
    )
    parser.add_argument(
        "-H",
        "--host",
        type=str,
        default="localhost",
        dest="redis_host",
        help="Redis host",
    )
    parser.add_argument(
        "-n", "--iterations", type=int, default=1000, help="Iterations per benchmark"
    )
    parser.add_argument(
        "--cache-only", action="store_true", help="Run only cache hit/miss benchmarks"
    )
    parser.add_argument(
        "--compare-only",
        action="store_true",
        help="Run only JMESPath vs JSONPath comparison",
    )
    args = parser.parse_args()

    r = redis.Redis(host=args.redis_host, port=args.port, decode_responses=True)

    # Check connection
    try:
        r.ping()
    except redis.ConnectionError:
        print(f"Error: Cannot connect to Redis at {args.redis_host}:{args.port}")
        return 1

    # Check if JMESPath is available
    try:
        r.execute_command("JSON.SET", "test", "$", '{"a":1}')
        r.execute_command("JSON.JMESPATH", "test", "a")
    except redis.ResponseError as e:
        print(f"Error: JMESPath not available: {e}")
        return 1

    print("JMESPath Expression Caching Benchmark")
    print(f"Redis: {args.redis_host}:{args.port}")
    print(f"Iterations per test: {args.iterations}")

    # Set up test data
    print("\nSetting up test data...")
    setup_test_data(r)

    if args.cache_only:
        run_cache_hit_benchmark(r, args.iterations)
        run_cache_miss_benchmark(r, args.iterations)
    elif args.compare_only:
        run_jsonpath_comparison(r, args.iterations)
    else:
        run_cache_hit_benchmark(r, args.iterations)
        run_cache_miss_benchmark(r, args.iterations)
        run_custom_function_benchmark(r, args.iterations)
        run_jsonpath_comparison(r, args.iterations)
        run_transformation_benchmark(r, args.iterations)

    print("\n" + "=" * 60)
    print("Benchmark complete!")
    print("=" * 60)

    return 0


if __name__ == "__main__":
    exit(main())
