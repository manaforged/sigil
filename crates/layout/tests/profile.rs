//! Layout engine records profiler counters on compute.

#![allow(unused_crate_dependencies)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_hal as _;
use smallvec as _;

mod stub;
use stub::stub_engine;

use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;
use sigil_profile::{disable, enable, frames, reset, tick_frame};
use sigil_reactive::{Runtime, signal};
use sigil_style::NodeStyleExt;

/// Serialize tests that mutate the process-global profiler store
/// (enable/reset/disable) — same convention as sigil-profile's own tests.
fn profile_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn compute_records_layout_miss() {
    let _gate = profile_lock();
    enable();
    reset();
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(text("hi")));
    cx.get_mut(root)
        .expect("root")
        .attach(LayoutStyle::flex_row().fixed_size(200.0, 40.0));
    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(200.0, 40.0))
        .expect("compute");
    tick_frame();
    let recorded = frames();
    let last = recorded.last().expect("frame");
    assert!(
        last.counter("layout.miss") >= 1,
        "first compute is a cache miss"
    );
    disable();
}

#[test]
fn second_compute_is_a_cache_hit() {
    let _gate = profile_lock();
    enable();
    reset();
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .child(text("hi"))
            .layout(LayoutStyle::flex_row().fixed_size(200.0, 40.0)),
    );
    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(200.0, 40.0))
        .expect("first");
    cx.clear_dirty();
    engine
        .compute(&mut cx, root, Size::fixed(200.0, 40.0))
        .expect("second");
    let generation = engine.generation();
    engine
        .compute(&mut cx, root, Size::fixed(200.0, 40.0))
        .expect("third");
    assert_eq!(
        engine.generation(),
        generation,
        "unchanged clean frame is a cache hit"
    );
    disable();
}

#[test]
fn mark_mutated_without_edit_is_a_cache_hit() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .child(text("hi"))
            .layout(LayoutStyle::flex_row().fixed_size(200.0, 40.0)),
    );
    let mut engine = stub_engine();
    let avail = Size::fixed(200.0, 40.0);
    engine.compute(&mut cx, root, avail).expect("first");
    let generation = engine.generation();
    cx.mark_mutated();
    engine.compute(&mut cx, root, avail).expect("mutated");
    assert_eq!(
        engine.generation(),
        generation,
        "mark_mutated without a node edit does not re-layout"
    );
}

#[test]
fn hidden_subtree_is_not_measured() {
    use std::cell::Cell;
    use std::rc::Rc;

    struct CountMeasure {
        hits: Rc<Cell<u32>>,
    }
    impl TextMeasure for CountMeasure {
        fn measure(&self, _: &sigil_kernel::TextContent, _: Size) -> sigil_hal::LogicalSize {
            self.hits.set(self.hits.get() + 1);
            sigil_hal::LogicalSize::new(40.0, 16.0)
        }
    }

    Runtime::scope("layout-hide-measure", || {
        let show = signal(true);
        let hits = Rc::new(Cell::new(0));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(200.0, 80.0))
                .child(container().visible_when(show).child(text("secret"))),
        );
        let mut engine = LayoutEngine::with_text_measure(Box::new(CountMeasure {
            hits: Rc::clone(&hits),
        }));
        let avail = Size::fixed(200.0, 80.0);
        engine.compute(&mut cx, root, avail).expect("show");
        let shown = hits.get();
        assert!(shown >= 1, "visible text is measured");
        show.set(false);
        hits.set(0);
        engine.compute(&mut cx, root, avail).expect("hide");
        assert_eq!(hits.get(), 0, "display:none subtree is not measured");
        hits.set(0);
        engine.compute(&mut cx, root, avail).expect("stay hidden");
        let generation = engine.generation();
        hits.set(0);
        engine.compute(&mut cx, root, avail).expect("settle");
        assert_eq!(
            engine.generation(),
            generation,
            "unchanged hidden frame is a cache hit"
        );
        assert_eq!(hits.get(), 0, "settled hidden frame does not remasure");
    });
}

#[test]
fn second_engine_does_not_inherit_text_memo() {
    use std::cell::Cell;
    use std::rc::Rc;

    struct CountMeasure {
        hits: Rc<Cell<u32>>,
    }
    impl TextMeasure for CountMeasure {
        fn measure(&self, _: &sigil_kernel::TextContent, _: Size) -> sigil_hal::LogicalSize {
            self.hits.set(self.hits.get() + 1);
            sigil_hal::LogicalSize::new(40.0, 16.0)
        }
    }

    Runtime::scope("layout-memo-isolation", || {
        let hits = Rc::new(Cell::new(0));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(80.0, 24.0))
                .child(text("hello")),
        );
        let avail = Size::fixed(80.0, 24.0);
        let mut first = LayoutEngine::with_text_measure(Box::new(CountMeasure {
            hits: Rc::clone(&hits),
        }));
        first.compute(&mut cx, root, avail).expect("first");
        assert!(hits.get() >= 1, "first engine measures");
        hits.set(0);
        let mut second = LayoutEngine::with_text_measure(Box::new(CountMeasure {
            hits: Rc::clone(&hits),
        }));
        second.compute(&mut cx, root, avail).expect("second");
        assert!(
            hits.get() >= 1,
            "new engine must remasure; TEXT_MEMO must not leak across LayoutDrive::new"
        );
    });
}

#[test]
fn dirty_parent_reuses_text_memo() {
    use std::cell::Cell;
    use std::rc::Rc;

    struct CountMeasure {
        hits: Rc<Cell<u32>>,
    }
    impl TextMeasure for CountMeasure {
        fn measure(&self, _: &sigil_kernel::TextContent, _: Size) -> sigil_hal::LogicalSize {
            self.hits.set(self.hits.get() + 1);
            sigil_hal::LogicalSize::new(40.0, 16.0)
        }
    }

    Runtime::scope("layout-memo-reuse", || {
        let hits = Rc::new(Cell::new(0));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(80.0, 24.0))
                .child(text("hello")),
        );
        let avail = Size::fixed(80.0, 24.0);
        let mut engine = LayoutEngine::with_text_measure(Box::new(CountMeasure {
            hits: Rc::clone(&hits),
        }));
        engine.compute(&mut cx, root, avail).expect("first");
        assert!(hits.get() >= 1, "first compute measures");
        hits.set(0);
        let _ = cx.get_mut(root);
        engine.compute(&mut cx, root, avail).expect("dirty parent");
        assert_eq!(hits.get(), 0, "unchanged text hits TEXT_MEMO");
    });
}

#[test]
fn nowrap_fixed_reactive_text_is_layout_hit() {
    Runtime::scope("layout-nowrap-tick", || {
        let label = signal(String::from("alpha"));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(80.0, 24.0))
                .child(
                    text(label)
                        .nowrap()
                        .layout(LayoutStyle::block().fixed_size(80.0, 24.0)),
                ),
        );
        let avail = Size::fixed(80.0, 24.0);
        let mut engine = stub_engine();
        engine.compute(&mut cx, root, avail).expect("first");
        let before = engine.generation();
        label.set(String::from("omega"));
        engine.compute(&mut cx, root, avail).expect("tick");
        assert_eq!(
            engine.generation(),
            before,
            "nowrap fixed-width reactive text must not remasure"
        );
    });
}

#[test]
fn paint_only_tick_on_wide_tree_is_hit() {
    Runtime::scope("layout-wide-tick", || {
        let label = signal(String::from("alpha"));
        let mut cx = Cx::new_root();
        let mut tree = container().layout(LayoutStyle::flex_column().fixed_size(200.0, 4000.0));
        for _ in 0..80 {
            tree = tree.child(container().layout(LayoutStyle::block().fixed_size(200.0, 24.0)));
        }
        tree = tree.child(
            text(label)
                .nowrap()
                .layout(LayoutStyle::block().fixed_size(80.0, 24.0)),
        );
        let root = cx.add(tree);
        let avail = Size::fixed(200.0, 4000.0);
        let mut engine = stub_engine();
        engine.compute(&mut cx, root, avail).expect("first");
        let before = engine.generation();
        label.set(String::from("omega"));
        engine.compute(&mut cx, root, avail).expect("tick");
        assert_eq!(
            engine.generation(),
            before,
            "paint-only tick on a wide static tree must be a cache hit"
        );
    });
}
