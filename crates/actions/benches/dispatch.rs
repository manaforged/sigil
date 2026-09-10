#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;

use std::any::Any;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil_actions::{Action, ActionRegistry, HandlerKey};

#[derive(Clone)]
struct Ping;

impl Action for Ping {
    fn debug_name(&self) -> &'static str {
        "Ping"
    }
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn build_chain(depth: u64) -> Vec<HandlerKey> {
    (1..=depth).map(HandlerKey::from_raw).collect()
}

fn bench_dispatch(c: &mut Criterion) {
    c.bench_function("dispatch_chain_1", |b| {
        let mut registry = ActionRegistry::new();
        let counter = Rc::new(Cell::new(0_u64));
        let c2 = Rc::clone(&counter);
        registry.register::<Ping>(HandlerKey::from_raw(1), move |_, _| {
            c2.set(c2.get() + 1);
        });
        let chain = build_chain(1);
        b.iter(|| {
            let r = registry.dispatch(black_box(&Ping), black_box(&chain));
            black_box(r);
        });
    });

    c.bench_function("dispatch_chain_10", |b| {
        let mut registry = ActionRegistry::new();
        registry.register::<Ping>(HandlerKey::from_raw(99), |_, _| {});
        let chain = build_chain(10);
        b.iter(|| {
            let r = registry.dispatch(black_box(&Ping), black_box(&chain));
            black_box(r);
        });
    });

    c.bench_function("dispatch_chain_10_handled", |b| {
        let mut registry = ActionRegistry::new();
        let counter = Rc::new(Cell::new(0_u64));
        let c2 = Rc::clone(&counter);
        registry.register::<Ping>(HandlerKey::from_raw(10), move |_, _| {
            c2.set(c2.get() + 1);
        });
        let chain = build_chain(10);
        b.iter(|| {
            let r = registry.dispatch(black_box(&Ping), black_box(&chain));
            black_box(r);
        });
    });

    c.bench_function("dispatch_chain_100_handled", |b| {
        let mut registry = ActionRegistry::new();
        let counter = Rc::new(Cell::new(0_u64));
        let c2 = Rc::clone(&counter);
        registry.register::<Ping>(HandlerKey::from_raw(100), move |_, _| {
            c2.set(c2.get() + 1);
        });
        let chain = build_chain(100);
        b.iter(|| {
            let r = registry.dispatch(black_box(&Ping), black_box(&chain));
            black_box(r);
        });
    });
}

criterion_group!(benches, bench_dispatch);
criterion_main!(benches);
