//! Benchmarks for JMESPath expression caching.
//!
//! Run with: cargo bench --features jmespath -p redis_json

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

#[cfg(feature = "jmespath")]
mod jmespath_benches {
    use super::*;

    // We need to test the caching mechanism directly
    // Since the cache is thread-local in jmespath_query, we'll benchmark
    // the public query function behavior

    use jmespath::{Expression, Variable};
    use serde_json::json;
    use std::sync::Arc;

    /// Benchmark parsing expressions (simulates cache miss)
    pub fn bench_expression_parsing(c: &mut Criterion) {
        let expressions = vec![
            "name",
            "[*].name",
            "[?age > `30`].name",
            "users | sort_by(@, &age) | [0:5]",
            "{name: name, age: age, active: active}",
        ];

        let mut group = c.benchmark_group("jmespath_parsing");

        for expr in expressions {
            group.bench_with_input(BenchmarkId::new("parse", expr), expr, |b, expr| {
                b.iter(|| {
                    let parsed = jmespath::compile(black_box(expr)).unwrap();
                    black_box(parsed)
                });
            });
        }

        group.finish();
    }

    /// Benchmark searching with pre-compiled expressions (simulates cache hit)
    pub fn bench_cached_search(c: &mut Criterion) {
        let small_doc = json!({"name": "Alice", "age": 30, "city": "NYC"});
        let small_var = Variable::from_serializable(&small_doc).unwrap();

        let users: Vec<_> = (0..100)
            .map(|i| {
                json!({
                    "name": format!("User{}", i),
                    "age": 20 + (i % 50),
                    "active": i % 2 == 0,
                    "score": i as f64 * 1.5
                })
            })
            .collect();
        let users_doc = json!(users);
        let users_var = Variable::from_serializable(&users_doc).unwrap();

        let mut group = c.benchmark_group("jmespath_cached_search");

        // Simple field access
        let expr = jmespath::compile("name").unwrap();
        group.bench_function("simple_field", |b| {
            b.iter(|| {
                let result = expr.search(black_box(&small_var)).unwrap();
                black_box(result)
            });
        });

        // Array projection
        let expr = jmespath::compile("[*].name").unwrap();
        group.bench_function("array_projection_100", |b| {
            b.iter(|| {
                let result = expr.search(black_box(&users_var)).unwrap();
                black_box(result)
            });
        });

        // Filter expression
        let expr = jmespath::compile("[?active].name").unwrap();
        group.bench_function("filter_100", |b| {
            b.iter(|| {
                let result = expr.search(black_box(&users_var)).unwrap();
                black_box(result)
            });
        });

        // Sort and slice
        let expr = jmespath::compile("[*] | sort_by(@, &age) | [0:10]").unwrap();
        group.bench_function("sort_slice_100", |b| {
            b.iter(|| {
                let result = expr.search(black_box(&users_var)).unwrap();
                black_box(result)
            });
        });

        group.finish();
    }

    /// Benchmark the overhead of cache lookup simulation
    pub fn bench_cache_overhead(c: &mut Criterion) {
        use std::collections::HashMap;

        let mut group = c.benchmark_group("cache_overhead");

        // Simulate LRU cache lookup with HashMap
        let mut cache: HashMap<String, Arc<Expression<'static>>> = HashMap::new();
        let expressions = vec![
            "name",
            "[*].name",
            "[?age > `30`].name",
            "users | sort_by(@, &age)",
        ];

        // Pre-populate cache
        for expr in &expressions {
            let compiled = jmespath::compile(expr).unwrap();
            cache.insert(expr.to_string(), Arc::new(compiled));
        }

        // Benchmark cache hit
        group.bench_function("cache_hit", |b| {
            b.iter(|| {
                let key = black_box("[*].name");
                let expr = cache.get(key);
                black_box(expr)
            });
        });

        // Benchmark cache miss + insert
        group.bench_function("cache_miss_compile", |b| {
            b.iter(|| {
                let key = black_box("new_expression.field");
                let compiled = jmespath::compile(key).unwrap();
                black_box(compiled)
            });
        });

        group.finish();
    }

    /// Benchmark different document sizes
    pub fn bench_document_sizes(c: &mut Criterion) {
        let mut group = c.benchmark_group("jmespath_doc_sizes");

        for size in [10, 100, 1000] {
            let users: Vec<_> = (0..size)
                .map(|i| {
                    json!({
                        "name": format!("User{}", i),
                        "age": 20 + (i % 50),
                        "active": i % 2 == 0
                    })
                })
                .collect();
            let doc = json!(users);
            let var = Variable::from_serializable(&doc).unwrap();

            let expr = jmespath::compile("[*].name").unwrap();
            group.bench_with_input(
                BenchmarkId::new("projection", size),
                &(expr, var),
                |b, (expr, var)| {
                    b.iter(|| {
                        let result = expr.search(black_box(var)).unwrap();
                        black_box(result)
                    });
                },
            );
        }

        group.finish();
    }
}

#[cfg(feature = "jmespath")]
criterion_group!(
    benches,
    jmespath_benches::bench_expression_parsing,
    jmespath_benches::bench_cached_search,
    jmespath_benches::bench_cache_overhead,
    jmespath_benches::bench_document_sizes,
);

#[cfg(feature = "jmespath")]
criterion_main!(benches);

#[cfg(not(feature = "jmespath"))]
fn main() {
    println!("JMESPath feature not enabled. Run with --features jmespath");
}
