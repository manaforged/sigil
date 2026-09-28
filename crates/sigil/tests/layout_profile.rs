#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

#[path = "layout_stub/mod.rs"]
mod stub;
use stub::stub_engine;

use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;
use sigil::profile::{disable, enable, frames, reset, tick_frame};
use sigil::reactive::{Runtime, derived, signal};
use sigil::style::NodeStyleExt;

static RECORDER: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn alone() -> std::sync::MutexGuard<'static, ()> {
    RECORDER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn compute_records_layout_miss() {
    let _recorder = alone();
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
        last.counter(sigil::profile::Counter::LayoutMiss) >= 1,
        "first compute is a cache miss"
    );
    disable();
}

#[test]
fn second_compute_is_a_cache_hit() {
    let _recorder = alone();
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
        fn measure(&self, _: &sigil::kernel::TextContent, _: Size) -> sigil::hal::LogicalSize {
            self.hits.set(self.hits.get() + 1);
            sigil::hal::LogicalSize::new(40.0, 16.0)
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
        fn measure(&self, _: &sigil::kernel::TextContent, _: Size) -> sigil::hal::LogicalSize {
            self.hits.set(self.hits.get() + 1);
            sigil::hal::LogicalSize::new(40.0, 16.0)
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
            "a new engine must remeasure; the text memo belongs to one forest"
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
        fn measure(&self, _: &sigil::kernel::TextContent, _: Size) -> sigil::hal::LogicalSize {
            self.hits.set(self.hits.get() + 1);
            sigil::hal::LogicalSize::new(40.0, 16.0)
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
fn derived_text_increment_records_layout_path() {
    let _recorder = alone();
    Runtime::scope("hello-counter-increment", || {
        enable();
        reset();
        let count = signal(0_i32);
        let label = derived(move || format!("Count: {}", count.get()));
        let mut cx = Cx::new_root();
        let mut col = container()
            .layout(LayoutStyle::flex_column().fixed_size(480.0, 800.0).gap(4.0))
            .child(text(label))
            .child(text("Increment"));
        for i in 0..40 {
            col = col.child(text(format!("row {i}")));
        }
        let root = cx.add(col);
        let mut engine = stub_engine();
        let avail = Size::fixed(480.0, 800.0);
        engine.compute(&mut cx, root, avail).expect("first");
        tick_frame();
        reset();
        let started = std::time::Instant::now();
        count.set(1);
        engine.compute(&mut cx, root, avail).expect("increment");
        let elapsed_us = started.elapsed().as_micros();
        tick_frame();
        let recorded = frames();
        let last = recorded.last().expect("frame");
        let full = last.counter(sigil::profile::Counter::LayoutFull);
        let incr = last.counter(sigil::profile::Counter::LayoutIncr);
        let resize = last.counter(sigil::profile::Counter::LayoutResize);
        assert!(
            full + incr + resize >= 1,
            "increment must layout; full={full} incr={incr} resize={resize} {elapsed_us}us"
        );
        assert_eq!(
            full, 0,
            "hello-counter Increment must not layout.full; incr={incr} resize={resize} {elapsed_us}us"
        );
        disable();
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

#[test]
fn letter_spacing_change_remeasures_text() {
    struct SpacingMeasure;
    impl TextMeasure for SpacingMeasure {
        fn measure(&self, _: &sigil::kernel::TextContent, _: Size) -> sigil::hal::LogicalSize {
            sigil::hal::LogicalSize::new(40.0, 16.0)
        }
        fn measure_styled(
            &self,
            _: &sigil::kernel::TextContent,
            style: &sigil::style::TextStyle,
            _: Size,
        ) -> sigil::hal::LogicalSize {
            sigil::hal::LogicalSize::new(style.letter_spacing_em.mul_add(100.0, 40.0), 16.0)
        }
    }
    use sigil::style::StyleAtom;
    static TIGHT: &[StyleAtom] = &[StyleAtom::LetterSpacing(0)];
    static LOOSE: &[StyleAtom] = &[StyleAtom::LetterSpacing(200)];

    Runtime::scope("layout-memo-letter-spacing", || {
        let loose = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .style_reactive(move || if loose.get() { LOOSE } else { TIGHT })
                .layout(LayoutStyle::flex_row().fixed_size(400.0, 24.0)),
        );
        let label = cx.add_child(root, text("hello"));
        let avail = Size::fixed(400.0, 24.0);
        let mut engine = LayoutEngine::with_text_measure(Box::new(SpacingMeasure));
        engine.compute(&mut cx, root, avail).expect("tight");
        let tight_w = label_width(&cx, label);
        loose.set(true);
        engine.compute(&mut cx, root, avail).expect("loose");
        assert!(
            label_width(&cx, label) > tight_w,
            "letter spacing change must reach the measurer"
        );
    });
}

fn label_width(cx: &Cx, id: NodeId) -> f32 {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result exists")
        .rect()
        .width
}
