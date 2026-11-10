#![allow(clippy::unwrap_used)]

use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil::profile::{Counter, Frame, disable, enable, frames, reset, tick_frame};
use sigil::reactive::Owner;
use sigil_kittest::FakeMeasure;

const VIEW: (f32, f32) = (400.0, 300.0);
const ROWS: usize = 60;

fn engine() -> LayoutEngine {
    LayoutEngine::with_text_measure(Box::new(FakeMeasure::fixed(8.0, 16.0)))
}

fn compute(cx: &mut Cx, layout: &mut LayoutEngine, root: NodeId) {
    layout
        .compute(cx, root, Size::fixed(VIEW.0, VIEW.1))
        .unwrap();
    tick_frame();
}

fn measured(run: impl FnOnce()) -> Frame {
    enable();
    reset();
    run();
    let frame = frames().last().cloned().unwrap();
    disable();
    frame
}

#[test]
fn a_wheel_step_moves_scroll_content_without_a_layout_solve() {
    let _recorder = crate::serial::profile();
    Owner::scope("sigil-scroll-translate", || {
        let offset = signal(0.0_f32);
        let mut rows = container().layout(LayoutStyle::flex_column().gap(Space::Xs));
        for index in 0..ROWS {
            rows = rows.child(text(format!("row {index}")));
        }
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(VIEW.0, VIEW.1))
                .child(scroll(rows).extent(2_000.0).fill().scroll_bind(offset)),
        );
        let mut layout = engine();
        compute(&mut cx, &mut layout, root);
        compute(&mut cx, &mut layout, root);
        let frame = measured(|| {
            offset.set(120.0);
            compute(&mut cx, &mut layout, root);
        });
        assert!(
            frame.counter(Counter::LayoutTranslate) >= 1,
            "the scroll offset moves the content by translation"
        );
        assert_eq!(
            frame.counter(Counter::LayoutRoots),
            0,
            "no subtree was solved"
        );
        assert_eq!(frame.counter(Counter::LayoutFull), 0, "no full layout ran");
    });
}

#[test]
fn changing_one_label_reuses_the_measurements_of_its_siblings() {
    let _recorder = crate::serial::profile();
    Owner::scope("sigil-intrinsic-reuse", || {
        let label = signal(String::from("short"));
        let mut list = container().layout(LayoutStyle::flex_column().gap(Space::Xxs));
        for index in 0..ROWS {
            let row = container()
                .layout(LayoutStyle::flex_row().gap(Space::Xs))
                .child(text(format!("name {index}")))
                .child(text(format!("value {index}")));
            list = list.child(if index == 7 {
                row.child(text(label))
            } else {
                row
            });
        }
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(VIEW.0, VIEW.1))
                .child(list),
        );
        let mut layout = engine();
        let first = measured(|| compute(&mut cx, &mut layout, root));
        compute(&mut cx, &mut layout, root);
        let changed = measured(|| {
            label.set(String::from("a much longer label"));
            compute(&mut cx, &mut layout, root);
        });
        let cold = first.counter(Counter::LayoutIntrinsicMiss);
        let warm = changed.counter(Counter::LayoutIntrinsicMiss);
        assert!(
            cold >= ROWS as u64,
            "the first layout measures every row, got {cold}"
        );
        assert!(
            warm * 10 < cold,
            "one changed label re-measured {warm} of {cold} items"
        );
    });
}
