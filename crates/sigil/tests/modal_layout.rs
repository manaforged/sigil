#![allow(clippy::unwrap_used)]

use sigil::Host;
use sigil::app::FrameHooks;
use sigil::events::Event;
use sigil::geometry::Point;
use sigil::geometry::Rect;
use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil::profile::{Counter, disable, enable, frames, reset, tick_frame};
use sigil::reactive::Owner;
use sigil_kittest::FakeMeasure;

const VIEW: (f32, f32) = (800.0, 600.0);

fn engine() -> LayoutEngine {
    LayoutEngine::with_text_measure(Box::new(FakeMeasure::fixed(60.0, 16.0)))
}

fn lines(count: Signal<usize>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(Space::Xs))
        .children_reactive(move || {
            (0..count.get())
                .map(|index| text(format!("line {index}")))
                .collect::<Vec<_>>()
        })
}

fn page(above: Signal<usize>, inside: Signal<usize>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fixed_size(VIEW.0, VIEW.1))
        .child(lines(above))
        .child(
            container()
                .layout(LayoutStyle::flex_column().padding(Space::Xl))
                .child(
                    modal(signal(true)).title("Edit").width(320.0).child(
                        container()
                            .layout(LayoutStyle::flex_column().fixed_size(280.0, 240.0))
                            .child(lines(inside)),
                    ),
                ),
        )
}

fn dialog(cx: &Cx, layout: &LayoutEngine, root: NodeId) -> Rect {
    let mut found = None;
    cx.visit_preorder(root, |id, node| {
        if node.a11y.role == Role::Dialog {
            found = layout.result(id).map(|result| result.rect());
        }
    });
    found.expect("dialog laid out")
}

fn settle(cx: &mut Cx, layout: &mut LayoutEngine, root: NodeId) {
    for _ in 0..2 {
        sigil::animation::tick(1.0);
        layout
            .compute(cx, root, Size::fixed(VIEW.0, VIEW.1))
            .unwrap();
        tick_frame();
    }
}

fn laid_out(above: usize, inside: usize) -> Rect {
    let mut cx = Cx::new_root();
    let root = cx.add(page(signal(above), signal(inside)));
    let mut layout = engine();
    settle(&mut cx, &mut layout, root);
    dialog(&cx, &layout, root)
}

fn overlay_passes(above: usize, inside: usize) -> (u64, Rect) {
    let above_signal = signal(2);
    let inside_signal = signal(2);
    let mut cx = Cx::new_root();
    let root = cx.add(page(above_signal, inside_signal));
    let mut layout = engine();
    settle(&mut cx, &mut layout, root);
    enable();
    reset();
    above_signal.set(above);
    inside_signal.set(inside);
    settle(&mut cx, &mut layout, root);
    let passes = frames()
        .iter()
        .map(|frame| frame.counter(Counter::LayoutOverlay))
        .sum();
    disable();
    (passes, dialog(&cx, &layout, root))
}

fn frame(host: &mut Host, event: Option<Event>) {
    let hooks = FrameHooks {
        input: event.map(|event| -> sigil::app::FrameInputHook<'_> {
            Box::new(move |target: &mut Host| {
                target.route(event);
                None
            })
        }),
        ..FrameHooks::default()
    };
    host.frame(VIEW, 1.0 / 60.0, hooks);
    tick_frame();
}

#[test]
fn scrolling_a_list_in_an_open_modal_does_not_lay_the_modal_out_again() {
    let _recorder = crate::serial::profile();
    Owner::scope("sigil-modal-layout-scroll", || {
        let offset = signal(0.0_f32);
        let rows: Vec<String> = (0..200).map(|index| format!("row {index}")).collect();
        let mut host = Host::new(HostMode::Pixel);
        host.mount(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(VIEW.0, VIEW.1))
                .child(
                    modal(signal(true)).title("Pick").width(320.0).child(
                        virtual_list(rows, |_, row: &String| container().child(text(row.clone())))
                            .row_height(24.0)
                            .viewport_height(240.0)
                            .viewport_width(280.0)
                            .scroll_bind(offset),
                    ),
                ),
        );
        for _ in 0..4 {
            frame(&mut host, None);
        }
        enable();
        reset();
        for _ in 0..10 {
            let wheel = Event::wheel(Point::new(VIEW.0 / 2.0, VIEW.1 / 2.0), -1.0);
            frame(&mut host, Some(wheel));
        }
        let passes: u64 = frames()
            .iter()
            .map(|frame| frame.counter(Counter::LayoutOverlay))
            .sum();
        disable();
        assert!(offset.peek() > 0.0, "the wheel missed the list");
        assert_eq!(passes, 0, "each scroll laid the whole modal out again");
    });
}

#[test]
fn a_change_above_a_modal_keeps_it_pinned_to_the_viewport() {
    let _recorder = crate::serial::profile();
    Owner::scope("sigil-modal-layout-above", || {
        let (passes, rect) = overlay_passes(12, 2);
        assert!(
            passes >= 1,
            "an ancestor moved but the modal kept its old frame"
        );
        assert_eq!(
            rect,
            laid_out(12, 2),
            "the modal followed its host, not the viewport"
        );
    });
}

fn bubble_version(cx: &Cx, root: NodeId) -> u32 {
    let mut version = None;
    cx.visit_preorder(root, |_, node| {
        if node.a11y.role == Role::Tooltip {
            version = Some(node.version());
        }
    });
    version.expect("tooltip bubble in the tree")
}

#[test]
fn a_hidden_tooltip_stays_untouched_while_other_content_changes() {
    Owner::scope("sigil-hidden-tooltip-steady", || {
        let count = signal(1);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(VIEW.0, VIEW.1))
                .child(tooltip("Version").child(button("About")))
                .child(lines(signal(120)))
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(280.0, 240.0))
                        .child(lines(count)),
                ),
        );
        let mut layout = engine();
        settle(&mut cx, &mut layout, root);
        count.set(2);
        settle(&mut cx, &mut layout, root);
        let before = bubble_version(&cx, root);
        for next in 3..6 {
            count.set(next);
            settle(&mut cx, &mut layout, root);
        }
        assert_eq!(
            bubble_version(&cx, root),
            before,
            "layout rewrote a hidden tooltip on every frame"
        );
    });
}
