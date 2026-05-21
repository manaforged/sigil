//! Criterion micro-bench for `ActionRegistry::dispatch`.
//!
//! Three measurement points target the latency cost of walking a focus
//! chain of varying depth, with and without a handler at the leaf:
//!
//! - `dispatch_chain_1` — single-entry chain, handler at leaf.
//! - `dispatch_chain_10` — depth-10 chain, no handler (worst-case walk).
//! - `dispatch_chain_10_handled` — depth-10 chain, handler at the leaf
//!   (fires immediately on the first iteration).
//! - `dispatch_chain_100_handled` — depth-100 chain, handler at the
//!   leaf (sanity check that handled-at-leaf is O(1) regardless of
//!   chain length).

#![allow(missing_docs)]

use actions_macros as _;
use smallvec as _;

use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;

use std::any::Any;

use actions::{Action, ActionRegistry, HandlerKey};
use criterion::{Criterion, criterion_group, criterion_main};

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
        // Handler registered on key=99 — outside the chain — so dispatch
        // walks the whole chain finding nothing.
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
