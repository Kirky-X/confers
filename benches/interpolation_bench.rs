//! Interpolation benchmark for confers configuration library.
//!
//! Measures interpolation performance for various patterns.

use confers::interpolation::{interpolate, interpolate_tracked};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

/// Create a resolver from a static slice of key-value pairs.
fn make_resolver<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |key| {
        vars.iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.to_string())
    }
}

/// Benchmark: Simple single variable interpolation
fn bench_interpolate_simple(c: &mut Criterion) {
    let vars = vec![("HOST", "localhost"), ("PORT", "8080")];
    let r = make_resolver(&vars);

    c.bench_function("interpolate_simple", |b| {
        b.iter(|| interpolate(black_box("Server: ${HOST}:${PORT}"), &r).unwrap());
    });
}

/// Benchmark: Multiple variables without nesting
fn bench_interpolate_multiple(c: &mut Criterion) {
    let vars = vec![
        ("HOST", "localhost"),
        ("PORT", "8080"),
        ("DB_HOST", "db.example.com"),
        ("DB_PORT", "5432"),
    ];
    let r = make_resolver(&vars);

    c.bench_function("interpolate_multiple", |b| {
        b.iter(|| interpolate(black_box("${HOST}:${PORT} - ${DB_HOST}:${DB_PORT}"), &r).unwrap());
    });
}

/// Benchmark: Default value interpolation
fn bench_interpolate_with_default(c: &mut Criterion) {
    let r = make_resolver(&[]);

    c.bench_function("interpolate_with_default", |b| {
        b.iter(|| {
            interpolate(
                black_box("${PORT:8080} ${HOST:localhost} ${PATH:/usr/bin}"),
                &r,
            )
            .unwrap()
        });
    });
}

/// Benchmark: URL in default value (complex parsing)
fn bench_interpolate_url_default(c: &mut Criterion) {
    let r = make_resolver(&[]);

    c.bench_function("interpolate_url_default", |b| {
        b.iter(|| interpolate(black_box("${URL:http://localhost:8080/api/v1}"), &r).unwrap());
    });
}

/// Benchmark: Nested interpolation
fn bench_interpolate_nested(c: &mut Criterion) {
    let vars = vec![("BASE", "localhost"), ("DOMAIN", "${BASE}")];
    let r = make_resolver(&vars);

    c.bench_function("interpolate_nested", |b| {
        b.iter(|| interpolate(black_box("${DOMAIN}:${PORT:8080}"), &r).unwrap());
    });
}

/// Benchmark: Deep nested interpolation
fn bench_interpolate_deep_nested(c: &mut Criterion) {
    let vars = vec![
        ("A", "value_a"),
        ("B", "${A}"),
        ("C", "${B}"),
        ("D", "${C}"),
        ("E", "${D}"),
    ];
    let r = make_resolver(&vars);

    c.bench_function("interpolate_deep_nested", |b| {
        b.iter(|| interpolate(black_box("${E}"), &r).unwrap());
    });
}

/// Benchmark: Large text with few interpolations
fn bench_interpolate_large_text_few_vars(c: &mut Criterion) {
    let vars = vec![("HOST", "localhost")];
    let r = make_resolver(&vars);
    let template = "This is a very long configuration string that contains multiple \
         lines of text describing the server setup. The host is ${HOST} \
         and this text is designed to simulate a real-world configuration \
         file with a lot of static content and few variable references.";

    c.bench_function("interpolate_large_text_few_vars", |b| {
        b.iter(|| interpolate(black_box(&template), &r).unwrap());
    });
}

/// Benchmark: No interpolation (pure string pass-through)
fn bench_interpolate_no_vars(c: &mut Criterion) {
    let r = make_resolver(&[]);

    c.bench_function("interpolate_no_vars", |b| {
        b.iter(|| {
            interpolate(
                black_box("This is a static string with no variables at all."),
                &r,
            )
            .unwrap()
        });
    });
}

/// Benchmark: Tracked interpolation overhead
fn bench_interpolate_tracked(c: &mut Criterion) {
    let vars = vec![("HOST", "localhost"), ("PORT", "8080")];
    let r = make_resolver(&vars);

    c.bench_function("interpolate_tracked", |b| {
        b.iter(|| interpolate_tracked(black_box("${HOST}:${PORT}"), &r, false).unwrap());
    });
}

/// Benchmark: Many variables in single string
fn bench_interpolate_many_vars(c: &mut Criterion) {
    // Use a hashmap-based resolver for many variables
    let vars: std::collections::HashMap<&str, &str> = [
        ("VAR0", "value0"),
        ("VAR1", "value1"),
        ("VAR2", "value2"),
        ("VAR3", "value3"),
        ("VAR4", "value4"),
        ("VAR5", "value5"),
        ("VAR6", "value6"),
        ("VAR7", "value7"),
        ("VAR8", "value8"),
        ("VAR9", "value9"),
        ("VAR10", "value10"),
        ("VAR11", "value11"),
        ("VAR12", "value12"),
        ("VAR13", "value13"),
        ("VAR14", "value14"),
        ("VAR15", "value15"),
        ("VAR16", "value16"),
        ("VAR17", "value17"),
        ("VAR18", "value18"),
        ("VAR19", "value19"),
    ]
    .into_iter()
    .collect();
    let r = move |key: &str| vars.get(key).map(|v| (*v).to_owned());
    let template = "${VAR0} ${VAR1} ${VAR2} ${VAR3} ${VAR4} ${VAR5} ${VAR6} ${VAR7} \
                    ${VAR8} ${VAR9} ${VAR10} ${VAR11} ${VAR12} ${VAR13} ${VAR14} \
                    ${VAR15} ${VAR16} ${VAR17} ${VAR18} ${VAR19}";

    c.bench_function("interpolate_many_vars_20", |b| {
        b.iter(|| interpolate(black_box(template), &r).unwrap());
    });
}

criterion_group!(
    benches,
    bench_interpolate_simple,
    bench_interpolate_multiple,
    bench_interpolate_with_default,
    bench_interpolate_url_default,
    bench_interpolate_nested,
    bench_interpolate_deep_nested,
    bench_interpolate_large_text_few_vars,
    bench_interpolate_no_vars,
    bench_interpolate_tracked,
    bench_interpolate_many_vars,
);
criterion_main!(benches);
