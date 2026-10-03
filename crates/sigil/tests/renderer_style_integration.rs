#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::float_cmp, clippy::unwrap_used)]

use sigil::geometry::Size;
use sigil::kernel::*;
use sigil::layout::{LayoutBuilderExt, LayoutEngine, LayoutResult, LayoutStyle};
use sigil::scene::{
    CommandScene, DrawGlyphRun, DrawRect, SceneCommand, StrokeAlign, StrokeStyle, emit_scene,
};
use sigil::style::{
    Elevation, FocusRing, FontFamily, NodeStyleExt, Paint, Radius, TextLineHeight, Theme, Token,
};
use sigil_kittest::FixedMeasure;

#[test]
fn theme_accent_rect_emits_in_scene() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(120.0, 48.0))
            .bg(Token::Accent),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let rect = first_rect(&scene).unwrap();

    assert_eq!(rect.node, Some(root));
    assert_eq!(rect.rect.width, 120.0);
    assert_eq!(rect.rect.height, 48.0);
    assert_eq!(rect.color, Theme::default().accent);
}

#[test]
fn text_emits_default_on_surface_color() {
    let mut cx = Cx::new_root();
    let root = cx.add(text("Readable"));

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let text = find_text(&scene, "Readable").unwrap();

    assert_eq!(text.run.node, Some(root));
    assert_eq!(text.run.text, "Readable");
    assert_eq!(text.run.color, Theme::default().on_surface);
}

#[test]
fn styled_text_color_overrides_default() {
    let mut cx = Cx::new_root();
    let root = cx.add(text("Accent").text_color(Token::Accent));

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let text = find_text(&scene, "Accent").unwrap();

    assert_eq!(text.run.color, Theme::default().accent);
}

#[test]
fn styled_text_typography_emits_in_scene() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        text("Large")
            .font_size(20.0)
            .font_weight(700)
            .line_height_px(32.0),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let text = find_text(&scene, "Large").unwrap();

    assert_eq!(text.run.style.font_size, 20.0);
    assert_eq!(text.run.style.font_weight, 700);
    assert_eq!(text.run.style.line_height, TextLineHeight::Absolute(32.0));
}

#[test]
fn visible_focus_ring_emits_one_rounded_outside_stroke() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 40.0))
            .rounded(Radius::Lg)
            .focus_ring(FocusRing::static_visible(true)),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let rects = rects_for_node(&scene, root);

    assert_eq!(rects.len(), 1);
    let first = rects.first().unwrap();
    assert_eq!(first.rect.origin.x, -2.0);
    assert_eq!(first.rect.origin.y, -2.0);
    assert_eq!(first.rect.width, 104.0);
    assert_eq!(first.rect.height, 44.0);
    assert_eq!(first.border_radius.0, [10.0; 4]);
    assert_eq!(first.color.a, 0);

    let stroke = first.stroke.expect("focus ring has an outside stroke");
    assert_eq!(stroke.color, Theme::default().accent);
    assert_eq!(stroke.width, 2.0);
    assert_eq!(stroke.style, StrokeStyle::Solid);
    assert_eq!(stroke.align, StrokeAlign::Outside);
}

#[test]
fn nested_text_scene_rect_is_absolute() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(240.0, 120.0).padding(24.0))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(120.0, 60.0).padding(8.0))
                    .child(text("Nested")),
            ),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let text = find_text(&scene, "Nested").unwrap();

    assert_eq!(text.run.rect.origin.x, 32.0);
    assert_eq!(text.run.rect.origin.y, 32.0);
}

#[test]
fn shadow_class_parses_cleanly_and_paints() {
    let before = sigil::style::unknown_class_warning_count();
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(32.0, 32.0))
            .bg(Token::Surface)
            .shadow(Elevation::Xl),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let has_shadow = scene
        .commands
        .iter()
        .any(|command| matches!(command, sigil::scene::SceneCommand::DrawShadow { .. }));

    assert_eq!(
        sigil::style::unknown_class_warning_count(),
        before,
        "shadow-* now parses cleanly — no unknown-class diagnostic"
    );
    assert!(has_shadow, "shadow-xl must emit a DrawShadow command");
}

fn layout(cx: &mut Cx, root: NodeId) {
    let mut engine = LayoutEngine::with_text_measure(Box::new(FixedMeasure::default()));
    engine
        .compute(cx, root, Size::fixed(320.0, 240.0))
        .expect("layout compute succeeds");
    assert!(
        cx.get(root)
            .and_then(|node| node.get_extension::<LayoutResult>())
            .is_some()
    );
}

fn first_rect(scene: &CommandScene) -> Option<&DrawRect> {
    scene.commands.iter().find_map(|command| match command {
        SceneCommand::Rect(rect) => Some(rect),
        _ => None,
    })
}

fn rects_for_node(scene: &CommandScene, node: NodeId) -> Vec<&DrawRect> {
    scene
        .commands
        .iter()
        .filter_map(|command| match command {
            SceneCommand::Rect(rect) if rect.node == Some(node) => Some(rect),
            _ => None,
        })
        .collect()
}

fn find_text<'a>(scene: &'a CommandScene, expected: &str) -> Option<&'a DrawGlyphRun> {
    scene.find_text(expected)
}

#[test]
fn container_text_class_cascades_to_child_glyphs() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(120.0, 48.0))
            .text_color(Token::Accent)
            .child(text("Doom")),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let run = find_text(&scene, "Doom").expect("child glyph run emitted");
    assert_eq!(
        run.run.color,
        Theme::default().accent,
        "a bare text child must inherit the nearest ancestor's text-* class \
         (pre-cascade it silently painted theme.on_surface — the chip-tone \
         ghost-pass and the turn-strip WCAG blocker)"
    );
}

#[test]
fn container_font_classes_cascade_to_child_glyph_style() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(120.0, 48.0))
            .font_size(18.0)
            .font_weight(700)
            .font_family(FontFamily::Mono)
            .line_height_px(24.0)
            .child(text("Doom")),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let run = find_text(&scene, "Doom").expect("child glyph run emitted");
    assert_eq!(
        run.run.style.font_size, 18.0,
        "a bare text child must inherit the ancestor's text-lg (DR-056)"
    );
    assert_eq!(run.run.style.font_weight, 700, "font-bold must inherit");
    assert_eq!(run.run.style.font_family, "mono", "font-mono must inherit");
    assert_eq!(
        run.run.style.line_height,
        TextLineHeight::Absolute(24.0),
        "leading-24 must inherit"
    );
}

#[test]
fn own_font_class_beats_inherited_typography() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(120.0, 48.0))
            .font_size(18.0)
            .font_weight(700)
            .child(text("Own").font_size(12.0)),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let run = find_text(&scene, "Own").expect("child glyph run emitted");
    assert_eq!(
        run.run.style.font_size, 12.0,
        "a text node's own text-* size must beat the inherited one"
    );
    assert_eq!(
        run.run.style.font_weight, 700,
        "properties the node does not set itself keep the inherited value"
    );
}

#[test]
fn deferred_top_layer_subtree_keeps_inherited_typography() {
    let mut cx = Cx::new_root();
    let popup = container()
        .layout(LayoutStyle::block().fixed_size(80.0, 24.0))
        .with(sigil::kernel::TopLayer)
        .child(text("Deferred"));
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(120.0, 48.0))
            .font_size(18.0)
            .font_weight(700)
            .text_color(Token::Accent)
            .child(popup),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let run = find_text(&scene, "Deferred").expect("deferred glyph run emitted");
    assert_eq!(
        run.run.style.font_size, 18.0,
        "top-layer deferral must not drop the inherited font size"
    );
    assert_eq!(run.run.style.font_weight, 700);
    assert_eq!(
        run.run.color,
        Theme::default().accent,
        "the color cascade must also survive the deferral"
    );
}

#[test]
fn own_text_class_beats_inherited_cascade() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(120.0, 48.0))
            .text_color(Token::Accent)
            .child(text("Own").text_color(Token::OnAccent)),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let run = find_text(&scene, "Own").expect("child glyph run emitted");
    assert_eq!(
        run.run.color,
        Theme::default().on_accent,
        "a text node's own text-* class must beat the inherited color"
    );
}

#[test]
fn top_layer_subtree_paints_after_later_siblings() {
    let mut cx = Cx::new_root();
    let floating = container()
        .with(TopLayer)
        .layout(LayoutStyle::block().fixed_size(100.0, 40.0))
        .bg(Token::Accent);
    let sibling = container()
        .layout(LayoutStyle::block().fixed_size(100.0, 40.0))
        .bg(Token::Error9);
    let root = cx.add(container().child(floating).child(sibling));

    layout(&mut cx, root);
    let scene = emit_scene(&cx, root);

    let theme = Theme::default();
    let accent_pos = scene
        .commands
        .iter()
        .position(|c| matches!(c, SceneCommand::Rect(r) if r.color == theme.accent))
        .expect("top-layer rect emitted");
    let sibling_pos = scene
        .commands
        .iter()
        .position(
            |c| matches!(c, SceneCommand::Rect(r) if r.color != theme.accent && r.node.is_some()),
        )
        .expect("sibling rect emitted");
    assert!(
        accent_pos > sibling_pos,
        "top-layer subtree must paint after later siblings \
         (accent at {accent_pos}, sibling at {sibling_pos})"
    );
}
