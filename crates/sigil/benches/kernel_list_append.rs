#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::kernel::{Cx, for_each, text};
use sigil::reactive::{Owner, signal};

const ITEMS: u32 = 1000;

fn bench_list_append(c: &mut Criterion) {
    Owner::scope("bench_list_append", || {
        let items = signal((0..ITEMS).collect::<Vec<u32>>());
        let mut cx = Cx::new_root();
        let list = cx.add(for_each(
            items,
            |item: &u32| *item,
            |item: &u32| text(format!("item {item}")),
        ));
        cx.flush_reactive_children();
        c.bench_function("list_append/1000", |b| {
            b.iter(|| {
                items.update(|all| all.push(ITEMS));
                cx.flush_reactive_children();
                items.update(|all| {
                    all.pop();
                });
                cx.flush_reactive_children();
                black_box(cx.len());
            });
        });
        black_box(list);
    });
}

criterion_group!(benches, bench_list_append);
criterion_main!(benches);
