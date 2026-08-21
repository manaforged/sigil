#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Perf baseline for the I-T5 fuzzy matcher.
//!
//! Targets (W4.5):
//!   * 10k entries, 5-char needle: <= 5 ms / iteration
//!   * 100k entries, 5-char needle: <= 50 ms / iteration
//!
//! Run with `cargo bench -p sigil-ide-core --bench fuzzy_perf`.

#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil_ide_core::search::fuzzy;

// `smallvec` and `proptest` are listed as deps (smallvec via the
// runtime crate, proptest dev-only). The bench binary inherits the
// dep edges so silence the unused-crate lint.

fn make_haystack(n: usize) -> Vec<String> {
    // Deterministic synthetic file-path corpus mimicking a large
    // monorepo: `crate-<i>/src/module_<j>/file_<k>.rs` shapes.
    let mut v = Vec::with_capacity(n);
    let mut i: usize = 0;
    while v.len() < n {
        let crate_id = i % 200;
        let module_id = (i / 200) % 80;
        let file_id = i / (200 * 80);
        v.push(format!(
            "crate{crate_id:03}/src/module_{module_id:02}/file_{file_id:04}.rs"
        ));
        i += 1;
    }
    v
}

fn corpus_borrows(strings: &[String]) -> Vec<&str> {
    strings.iter().map(String::as_str).collect()
}

fn bench_10k(c: &mut Criterion) {
    let owned = make_haystack(10_000);
    let hay = corpus_borrows(&owned);
    c.bench_function("fuzzy_10k_needle5", |b| {
        b.iter(|| {
            let out = fuzzy(black_box("mfile"), black_box(&hay));
            black_box(out);
        });
    });
}

fn bench_100k(c: &mut Criterion) {
    let owned = make_haystack(100_000);
    let hay = corpus_borrows(&owned);
    c.bench_function("fuzzy_100k_needle5", |b| {
        b.iter(|| {
            let out = fuzzy(black_box("mfile"), black_box(&hay));
            black_box(out);
        });
    });
}

fn bench_short_needle(c: &mut Criterion) {
    let owned = make_haystack(10_000);
    let hay = corpus_borrows(&owned);
    c.bench_function("fuzzy_10k_needle2", |b| {
        b.iter(|| {
            let out = fuzzy(black_box("mr"), black_box(&hay));
            black_box(out);
        });
    });
}

criterion_group!(benches, bench_10k, bench_100k, bench_short_needle);
criterion_main!(benches);
