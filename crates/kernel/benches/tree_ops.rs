#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::{Criterion, criterion_group, criterion_main};
use sigil_kernel::{Cx, NodeBuilder, container, text};

fn tree_with_text_children(count: usize) -> NodeBuilder {
    let mut builder = container();
    for i in 0..count {
        builder = builder.child(text(format!("node {i}")));
    }
    builder
}

fn bench_add_1000_nodes(c: &mut Criterion) {
    c.bench_function("tree_add_1000_text_nodes", |b| {
        b.iter_custom(|iters| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iters {
                let mut cx = Cx::new_root();
                let tree = tree_with_text_children(1000);
                let start = Instant::now();
                let root = cx.add(tree);
                elapsed += start.elapsed();
                black_box(root);
                black_box(cx.len());
            }
            elapsed
        });
    });
}

fn bench_remove_100_node_subtree(c: &mut Criterion) {
    c.bench_function("tree_remove_100_node_subtree", |b| {
        b.iter_custom(|iters| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iters {
                let mut cx = Cx::new_root();
                let root = cx.add(tree_with_text_children(100));
                let start = Instant::now();
                black_box(cx.remove(root));
                elapsed += start.elapsed();
                black_box(cx.len());
            }
            elapsed
        });
    });
}

fn bench_walk_1000_nodes(c: &mut Criterion) {
    c.bench_function("tree_walk_1000_nodes", |b| {
        let mut cx = Cx::new_root();
        let root = cx.add(tree_with_text_children(1000));

        b.iter(|| {
            let mut seen = 0_usize;
            cx.visit_preorder(root, |id, node| {
                black_box(id);
                black_box(&node.kind);
                seen += 1;
            });
            black_box(seen);
        });
    });
}

criterion_group!(
    benches,
    bench_add_1000_nodes,
    bench_remove_100_node_subtree,
    bench_walk_1000_nodes,
);
criterion_main!(benches);
