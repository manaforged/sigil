#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::kernel::Cx;
use sigil::kernel::TextBuilder;
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil::scene::SceneCommand;

const LONG: &str = "an over-long single-line value that cannot fit a ninety-six pixel box";

fn clip_for(cell: TextBuilder) -> (Option<sigil::geometry::Rect>, sigil::geometry::Rect) {
    let mut out = (
        None,
        sigil::geometry::Rect::new(sigil::geometry::Point::new(0.0, 0.0), 0.0, 0.0),
    );
    sigil::reactive::Runtime::scope("text-clip", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(200.0, 40.0))
                .child(cell),
        );
        let mut layout =
            LayoutEngine::with_text_measure(Box::new(sigil::app::DefaultTextMeasure::new()));
        layout
            .compute(&mut cx, root, Size::fixed(200.0, 40.0))
            .expect("layout");
        let text_id = *cx.get(root).unwrap().children().first().unwrap();
        let rect = layout.result(text_id).unwrap().rect();
        let scene = sigil::scene::emit_scene(&cx, root);
        let mut clip: Option<sigil::geometry::Rect> = None;
        let mut hit = None;
        for command in &scene.commands {
            match command {
                SceneCommand::PushClip(push) => clip = Some(*push),
                SceneCommand::PopClip => clip = None,
                SceneCommand::GlyphRun(draw) if draw.run.node == Some(text_id) => hit = clip,
                _ => {}
            }
        }
        out = (hit, rect);
    });
    out
}

#[test]
fn ellipsize_in_a_box_too_narrow_for_the_ellipsis_clips() {
    let (clip, rect) = clip_for(
        text(LONG)
            .nowrap()
            .ellipsize()
            .layout(LayoutStyle::block().fixed_size(4.0, 40.0)),
    );
    assert_eq!(
        clip,
        Some(rect),
        "even the bare ellipsis is wider than 4px, so the run must clip to the box",
    );
}

#[test]
fn ellipsize_in_a_box_shorter_than_a_line_clips() {
    let (clip, rect) = clip_for(
        text(LONG)
            .nowrap()
            .ellipsize()
            .layout(LayoutStyle::block().fixed_size(96.0, 4.0)),
    );
    assert_eq!(
        clip,
        Some(rect),
        "the line is taller than the 4px box, so the run must clip to the box",
    );
}

#[test]
fn ellipsized_text_that_fits_emits_no_clip() {
    let (clip, _) = clip_for(
        text(LONG)
            .nowrap()
            .ellipsize()
            .layout(LayoutStyle::block().width(96.0)),
    );
    assert_eq!(
        clip, None,
        "an ellipsized run already truncated to its box needs no clip",
    );
}

#[test]
fn overflowing_nowrap_text_keeps_overflow_visible() {
    let (clip, _) = clip_for(
        text(LONG)
            .nowrap()
            .layout(LayoutStyle::block().fixed_size(96.0, 40.0)),
    );
    assert_eq!(
        clip, None,
        "non-ellipsized text keeps the historic overflow-visible paint (no clip)",
    );
}
