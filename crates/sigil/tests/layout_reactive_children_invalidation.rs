#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::float_cmp)]

#[path = "layout_stub/mod.rs"]
mod stub;
use stub::stub_engine;

use sigil::hal::Size;
use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;
use sigil::reactive::{Runtime, signal};

const AVAILABLE: Size = Size::fixed(800.0, 1000.0);

fn layout_rect(cx: &Cx, id: NodeId) -> sigil::hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result must exist")
        .rect()
}

#[test]
fn reactive_children_in_sized_container_only_relayouts_subtree() {
    Runtime::scope("layout_reactive_children_sized_container_subtree", || {
        let items = signal(vec!["a", "b"]);
        let mut cx = Cx::new_root();

        let inner_items = items;
        let outer = cx.add(
            container().layout(LayoutStyle::flex_column()).child(
                container()
                    .layout(LayoutStyle::flex_column().fixed_size(400.0, 600.0))
                    .child(
                        container()
                            .layout(LayoutStyle::flex_column())
                            .children_reactive(move || {
                                inner_items.get().into_iter().map(|_s| {
                                    container().layout(LayoutStyle::block().fixed_size(100.0, 20.0))
                                })
                            }),
                    ),
            ),
        );

        let mut engine = stub_engine();
        engine
            .compute(&mut cx, outer, AVAILABLE)
            .expect("initial compute succeeds");

        let gen_after_initial = engine.generation();
        let outer_rect_initial = layout_rect(&cx, outer);

        items.set(vec!["a", "b", "c", "d"]);

        engine
            .compute(&mut cx, outer, AVAILABLE)
            .expect("reactive compute succeeds");

        assert_ne!(
            engine.generation(),
            gen_after_initial,
            "engine generation must advance after children_reactive mutation"
        );

        let outer_rect_after = layout_rect(&cx, outer);
        assert_eq!(
            outer_rect_after.width.to_bits(),
            outer_rect_initial.width.to_bits(),
            "outer width must not change (containment root was middle)"
        );
        assert_eq!(
            outer_rect_after.height.to_bits(),
            outer_rect_initial.height.to_bits(),
            "outer height must not change (containment root was middle)"
        );
    });
}

#[test]
fn reactive_children_identical_set_is_a_cache_hit() {
    Runtime::scope("layout_reactive_children_identical_rebuild_cache", || {
        let items = signal(vec!["a", "b", "c"]);
        let mut cx = Cx::new_root();

        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(300.0, 400.0))
                .children_reactive(move || {
                    items
                        .get()
                        .into_iter()
                        .map(|_s| container().layout(LayoutStyle::block().fixed_size(100.0, 20.0)))
                }),
        );

        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("initial compute succeeds");

        let gen_after_initial = engine.generation();

        items.set(vec!["a", "b", "c"]);

        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("identical set compute succeeds");

        assert_eq!(
            engine.generation(),
            gen_after_initial,
            "equal Signal::set does not rebuild children or re-layout"
        );
    });
}

#[test]
fn reactive_children_dispatched_through_compute() {
    Runtime::scope("layout_reactive_children_via_compute", || {
        let items = signal(Vec::<&str>::new());
        let mut cx = Cx::new_root();

        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(300.0, 400.0))
                .children_reactive(move || {
                    items
                        .get()
                        .into_iter()
                        .map(|_s| container().layout(LayoutStyle::block().fixed_size(100.0, 20.0)))
                }),
        );

        let mut engine = stub_engine();

        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("initial compute succeeds");

        assert_eq!(
            cx.get(root).expect("root alive").children().len(),
            0,
            "no children in the initial empty build"
        );

        items.set(vec!["x", "y", "z"]);

        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("compute with queued rebuild succeeds");

        assert_eq!(
            cx.get(root).expect("root alive").children().len(),
            3,
            "compute must drain the reactive-children queue and materialise 3 children"
        );
    });
}
