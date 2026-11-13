#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]

use crate::layout_stub as stub;
use sigil::layout::Space;
use sigil::widgets::Widget;
use stub::{layout_rect, stub_engine};

use sigil::geometry::{Constraint, Point, Rect, Size};
use sigil::kernel::*;
use sigil::layout::*;
use sigil::reactive::{Owner, signal};

#[test]
fn position_reactive_tracks_signal_with_stable_identity() {
    Owner::scope("layout_reactive_position", || {
        let pos = signal(Point::new(50.0, 30.0));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(800.0, 600.0))
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                        .offset(pos),
                ),
        );
        let node = *cx.get(root).unwrap().children().first().unwrap();

        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, Constraint::fixed(800.0, 600.0))
            .expect("initial compute");
        let r1 = layout_rect(&cx, node);
        assert_eq!(
            (r1.origin.x, r1.origin.y),
            (50.0, 30.0),
            "node sits at the signal's initial inset; got {:?}",
            (r1.origin.x, r1.origin.y),
        );

        pos.set(Point::new(120.0, 200.0));
        engine
            .compute(&mut cx, root, Constraint::fixed(800.0, 600.0))
            .expect("reposition compute");
        let r2 = layout_rect(&cx, node);
        assert_eq!(
            (r2.origin.x, r2.origin.y),
            (120.0, 200.0),
            "node follows the signal to its new inset; got {:?}",
            (r2.origin.x, r2.origin.y),
        );

        let node_again = *cx.get(root).unwrap().children().first().unwrap();
        assert_eq!(
            node, node_again,
            "node identity must be stable across reposition (capture-safe)",
        );
    });
}

#[test]
fn bounds_reactive_publishes_the_absolute_rect() {
    Owner::scope("layout_bounds_reactive", || {
        let bounds = signal(Rect::new(Point::new(0.0, 0.0), 0.0, 0.0));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .fixed_size(800.0, 600.0)
                        .padding(Space::raw(10.0, "test fixture")),
                )
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                        .track_bounds(bounds),
                ),
        );

        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, Constraint::fixed(800.0, 600.0))
            .expect("initial compute");
        assert_eq!(
            bounds.peek(),
            Rect::new(Point::new(10.0, 10.0), 100.0, 40.0),
            "the bound signal carries the absolute border box after compute",
        );
    });
}

#[test]
fn track_position_and_track_size_publish_origin_and_size() {
    Owner::scope("layout_track_position_and_size", || {
        let pos = signal(Point::new(0.0, 0.0));
        let size = signal(Size::new(0.0, 0.0));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .fixed_size(800.0, 600.0)
                        .padding(Space::raw(12.0, "test fixture")),
                )
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(140.0, 44.0))
                        .track_position(pos)
                        .track_size(size),
                ),
        );

        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, Constraint::fixed(800.0, 600.0))
            .expect("initial compute");
        assert_eq!(pos.peek(), Point::new(12.0, 12.0));
        assert_eq!(size.peek(), Size::new(140.0, 44.0));
    });
}

#[test]
fn reposition_translates_the_subtree_without_a_resolve() {
    Owner::scope("layout_reactive_translate", || {
        let pos = signal(Point::new(0.0, 0.0));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(800.0, 600.0))
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                        .offset(pos)
                        .child(container().layout(LayoutStyle::block().fixed_size(10.0, 10.0))),
                ),
        );
        let strip = *cx.get(root).unwrap().children().first().unwrap();
        let leaf = *cx.get(strip).unwrap().children().first().unwrap();
        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, Constraint::fixed(800.0, 600.0))
            .expect("initial compute");
        let _profile = crate::serial::profile();
        sigil::profile::enable();
        sigil::profile::reset();
        sigil::profile::begin_frame(sigil::profile::BackendStatus::new(
            sigil::profile::Backend::Cpu,
            sigil::profile::FallbackCause::None,
        ));
        pos.set(Point::new(0.0, -35.0));
        engine
            .compute(&mut cx, root, Constraint::fixed(800.0, 600.0))
            .expect("translate compute");
        sigil::profile::end_frame();
        let counts = sigil::profile::frames()
            .last()
            .map(|f| f.counts.clone())
            .unwrap_or_default();
        assert!(
            counts
                .iter()
                .any(|(name, _)| *name == sigil::profile::Counter::LayoutTranslate),
            "a position-only change must take the translate path; counts {counts:?}"
        );
        assert_eq!(layout_rect(&cx, strip).origin.y, -35.0);
        assert_eq!(layout_rect(&cx, leaf).origin.y, -35.0);
    });
}

#[test]
fn reposition_with_a_dirty_descendant_still_translates() {
    Owner::scope("layout_reactive_translate_dirty_child", || {
        let pos = signal(Point::new(0.0, 0.0));
        let label = signal(String::from("a"));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(800.0, 600.0))
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                        .offset(pos)
                        .child(
                            container()
                                .layout(LayoutStyle::block().fixed_size(50.0, 10.0))
                                .child(text(move || label.get())),
                        ),
                ),
        );
        let strip = *cx.get(root).unwrap().children().first().unwrap();
        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, Constraint::fixed(800.0, 600.0))
            .expect("initial compute");
        let _profile = crate::serial::profile();
        sigil::profile::enable();
        sigil::profile::reset();
        sigil::profile::begin_frame(sigil::profile::BackendStatus::new(
            sigil::profile::Backend::Cpu,
            sigil::profile::FallbackCause::None,
        ));
        label.set(String::from("bb"));
        pos.set(Point::new(0.0, -35.0));
        engine
            .compute(&mut cx, root, Constraint::fixed(800.0, 600.0))
            .expect("second compute");
        sigil::profile::end_frame();
        let counts = sigil::profile::frames()
            .last()
            .map(|f| f.counts.clone())
            .unwrap_or_default();
        assert!(
            counts
                .iter()
                .any(|(name, _)| *name == sigil::profile::Counter::LayoutTranslate),
            "the strip must translate even when a cell inside it is dirty; counts {counts:?}"
        );
        assert_eq!(layout_rect(&cx, strip).origin.y, -35.0);
    });
}

#[test]
fn incremental_root_with_margin_keeps_its_place() {
    Owner::scope("layout_margin_root", || {
        let label = signal(String::from("a"));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(400.0, 300.0))
                .child(container().layout(LayoutStyle::block().fixed_size(100.0, 20.0)))
                .child(
                    container()
                        .layout(
                            LayoutStyle::block()
                                .fixed_size(120.0, 40.0)
                                .margin(Space::raw(10.0, "test fixture")),
                        )
                        .child(text(move || label.get())),
                ),
        );
        let boxed = *cx.get(root).unwrap().children().get(1).unwrap();
        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, Constraint::fixed(400.0, 300.0))
            .expect("initial compute");
        let before = layout_rect(&cx, boxed);
        label.set(String::from("bb"));
        engine
            .compute(&mut cx, root, Constraint::fixed(400.0, 300.0))
            .expect("incremental compute");
        let after = layout_rect(&cx, boxed);
        assert_eq!(
            (after.origin.x, after.origin.y),
            (before.origin.x, before.origin.y),
            "a margin root must not drift when re-solved on its own"
        );
    });
}

#[test]
fn multiple_geometry_trackers_on_one_node_all_receive_updates() {
    Owner::scope("layout_multiple_geometry_trackers", || {
        let bounds_a = signal(sigil::geometry::Rect::default());
        let bounds_b = signal(sigil::geometry::Rect::default());
        let pos_a = signal(Point::ZERO);
        let pos_b = signal(Point::ZERO);
        let size_a = signal(sigil::geometry::Size::ZERO);
        let size_b = signal(sigil::geometry::Size::ZERO);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(400.0, 300.0))
                .child(container().layout(LayoutStyle::block().fixed_size(100.0, 24.0)))
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(120.0, 40.0))
                        .track_bounds(bounds_a)
                        .track_bounds(bounds_b)
                        .track_position(pos_a)
                        .track_position(pos_b)
                        .track_size(size_a)
                        .track_size(size_b),
                ),
        );
        let mut engine = stub_engine();
        engine
            .compute(&mut cx, root, Constraint::fixed(400.0, 300.0))
            .expect("compute");
        let expected_bounds = sigil::geometry::Rect::new(Point::new(0.0, 24.0), 120.0, 40.0);
        assert_eq!(bounds_a.get(), expected_bounds);
        assert_eq!(bounds_b.get(), expected_bounds);
        assert_eq!(pos_a.get(), Point::new(0.0, 24.0));
        assert_eq!(pos_b.get(), Point::new(0.0, 24.0));
        assert_eq!(size_a.get(), sigil::geometry::Size::new(120.0, 40.0));
        assert_eq!(size_b.get(), sigil::geometry::Size::new(120.0, 40.0));
    });
}
