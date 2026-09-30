#![allow(unused_crate_dependencies, clippy::unwrap_used)]

use sigil::geometry::{Point, Rect, Size};
use sigil::kernel::*;
use sigil::layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, Overflow};
use sigil::scene::{SceneCommand, emit_scene, emit_scene_clipped};
use sigil::style::{Paint, Token};
use sigil_kittest::CharMeasure;

const TEXT_MEASURE: CharMeasure = CharMeasure {
    char_width: 8.0,
    line_height: 20.0,
};

fn layout(cx: &mut Cx, root: NodeId) {
    let mut engine = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
    engine
        .compute(cx, root, Size::fixed(200.0, 80.0))
        .expect("layout");
}

fn glyph_runs(scene: &sigil::scene::CommandScene) -> usize {
    scene
        .commands
        .iter()
        .filter(|cmd| matches!(cmd, SceneCommand::GlyphRun(_)))
        .count()
}

fn column(overflow: Option<Overflow>) -> NodeBuilder {
    let mut style = LayoutStyle::flex_column().fixed_size(200.0, 80.0);
    if let Some(overflow) = overflow {
        style = style.overflow(overflow);
    }
    let mut col = container().layout(style);
    for index in 0..40 {
        col = col.child(
            text(format!("row {index}"))
                .layout(LayoutStyle::block().fixed_size(200.0, 20.0).shrink(0.0))
                .text_color(Token::OnSurface),
        );
    }
    col
}

#[test]
fn clipped_emit_skips_rows_below_the_viewport() {
    let mut cx = Cx::new_root();
    let root = cx.add(column(None));
    layout(&mut cx, root);

    let full = emit_scene(&cx, root);
    let clipped = emit_scene_clipped(
        &cx,
        root,
        Some(Rect::new(Point::new(0.0, 0.0), 200.0, 80.0)),
    );
    let full_n = glyph_runs(&full);
    let clipped_n = glyph_runs(&clipped);
    assert!(full_n >= 40, "unclipped emit walks every row, got {full_n}");
    assert!(
        clipped_n <= 12,
        "80px view + 64px overscan is ~7 rows, got {clipped_n}"
    );
    assert!(
        clipped_n < full_n / 2,
        "clip must cut more than half the rows ({clipped_n} vs {full_n})"
    );
}

#[test]
fn overflow_clip_culls_without_an_explicit_viewport() {
    let mut cx = Cx::new_root();
    let root = cx.add(column(Some(Overflow::Hidden)));
    layout(&mut cx, root);
    let n = glyph_runs(&emit_scene(&cx, root));
    assert!(
        n <= 6,
        "overflow hidden on the 80px column must drop offscreen rows, got {n}"
    );
}

#[test]
fn second_emit_matches_first() {
    let mut cx = Cx::new_root();
    let root = cx.add(column(None));
    layout(&mut cx, root);
    let clip = Some(Rect::new(Point::new(0.0, 0.0), 200.0, 80.0));
    let a = emit_scene_clipped(&cx, root, clip);
    let b = emit_scene_clipped(&cx, root, clip);
    assert_eq!(
        a.commands.len(),
        b.commands.len(),
        "cached emit must keep the same command count"
    );
}
