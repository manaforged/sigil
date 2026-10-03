#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::float_cmp)]

#[path = "events_support/input.rs"]
mod input;
#[path = "scene_support/mod.rs"]
mod scene_support;
#[path = "widget_support/mod.rs"]
mod widget_support;

use input::{pointer_down, pointer_move};
use scene_support::color;
use sigil::events::EventRouter;
use sigil::geometry::Size;
use sigil::kernel::{Cx, IntoNode, NodeBuilder, container, text};
use sigil::layout::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LayoutBuilderExt, LayoutStyle,
    LengthPercentage, Overflow,
};
use sigil::scene::{DrawRect, SceneCommand, emit_scene};
use sigil::style::{
    ColorAtom, ColorKey, ColorOrToken, FontFamily, Paint, Radius, Rgba8, StateStyleBuilder,
    StateVariants, TextLineHeight, Theme, Token,
};
use widget_support::compute;

const VIEWPORT: Size = Size::fixed(200.0, 100.0);
const REST: Rgba8 = color(240, 240, 240, 255);
const A: Rgba8 = color(200, 20, 20, 255);
const B: Rgba8 = color(20, 200, 20, 255);
const C: Rgba8 = color(20, 20, 200, 255);
const D: Rgba8 = color(90, 90, 90, 255);

#[derive(Clone, Copy)]
enum Drive {
    Rest,
    Hover,
    Press,
    Focus,
}

fn pane() -> NodeBuilder {
    container()
        .focusable(true)
        .layout(LayoutStyle::block().fixed_size(40.0, 40.0))
}

fn rects(build: impl IntoNode, drive: Drive) -> Vec<DrawRect> {
    let mut cx = Cx::new_root();
    let root = cx.add(build);
    let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
    let mut router = EventRouter::new();
    match drive {
        Drive::Rest => {}
        Drive::Hover => router.route(&mut cx, &layout, pointer_move(8.0, 8.0)),
        Drive::Press => {
            router.route(&mut cx, &layout, pointer_move(8.0, 8.0));
            router.route(&mut cx, &layout, pointer_down(8.0, 8.0));
        }
        Drive::Focus => router.set_focus(&mut cx, Some(root)),
    }
    emit_scene(&cx, root)
        .commands
        .into_iter()
        .filter_map(|command| match command {
            SceneCommand::Rect(rect) => Some(rect),
            _ => None,
        })
        .collect()
}

fn colors(build: impl IntoNode, drive: Drive) -> Vec<Rgba8> {
    rects(build, drive).iter().map(|rect| rect.color).collect()
}

fn rest_rect(build: impl IntoNode) -> DrawRect {
    rects(build, Drive::Rest)
        .into_iter()
        .next()
        .expect("a rect")
}

fn painted_opacity(build: impl IntoNode) -> Option<u16> {
    let mut cx = Cx::new_root();
    let root = cx.add(build);
    let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
    let mut layers = vec![1.0_f32];
    for command in emit_scene(&cx, root).commands {
        let top = *layers.last().unwrap();
        match command {
            SceneCommand::PushLayer(scope) => layers.push(top * scope.opacity),
            SceneCommand::SaveLayer { opacity } => layers.push(top * opacity),
            SceneCommand::PopLayer | SceneCommand::RestoreLayer => {
                layers.pop();
            }
            SceneCommand::Rect(rect) => {
                let scaled = (f32::from(rect.opacity) * top).round();
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "the value is clamped to the u16 opacity range first"
                )]
                return Some(scaled.clamp(0.0, 10_000.0) as u16);
            }
            _ => {}
        }
    }
    None
}

#[test]
fn default_impl_chains_correctly() {
    let node = container()
        .bg(Token::Surface)
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(80.0, 20.0)
                .padding_x(16.0),
        )
        .child(
            container()
                .bg(A)
                .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
        );
    let all = rects(node, Drive::Rest);
    assert_eq!(all[0].color, Theme::default().surface);
    let child = all.iter().find(|rect| rect.color == A).expect("child rect");
    assert_eq!(child.rect.origin.x, 16.0);
}

#[test]
fn hover_state_record_overwrites_on_second_call() {
    let hovered = colors(
        pane().bg(REST).hover(|s| s.bg(A)).hover(|s| s.bg(B)),
        Drive::Hover,
    );
    assert!(
        hovered.contains(&B),
        "hover slot must be last-call-wins: {hovered:?}"
    );
    assert!(
        !hovered.contains(&A),
        "the first hover call must be replaced: {hovered:?}"
    );
}

#[test]
fn hover_atoms_accumulate_within_closure() {
    let node = pane().bg(REST).hover(|s| s.bg(A).rounded(Radius::Md));
    let hovered = rects(node, Drive::Hover);
    let rect = hovered
        .iter()
        .find(|rect| rect.color == A)
        .expect("hover bg");
    assert_eq!(rect.border_radius.0[0], 6.0);
}

#[test]
fn focus_slot_records_atoms() {
    let focused = colors(pane().bg(REST).focus(|s| s.bg(A)), Drive::Focus);
    assert!(focused.contains(&A), "focus slot must paint: {focused:?}");
}

#[test]
fn pressed_slot_records_atoms() {
    let pressed = colors(pane().bg(REST).pressed(|s| s.bg(A)), Drive::Press);
    assert!(pressed.contains(&A), "pressed slot must paint: {pressed:?}");
}

#[test]
fn disabled_state_atoms_emit() {
    let node = || pane().bg(REST).disabled_style(|s| s.bg(A));
    assert_eq!(colors(node(), Drive::Rest), [REST]);
    assert!(!colors(node(), Drive::Hover).contains(&A));
}

#[test]
fn bg_accepts_rgba8() {
    assert_eq!(rest_rect(pane().bg(A)).color, A);
}

#[test]
fn bg_accepts_theme_token() {
    assert_eq!(
        rest_rect(pane().bg(Token::Surface)).color,
        Theme::default().surface
    );
}

#[test]
fn bg_accepts_color_atom() {
    assert_eq!(rest_rect(pane().bg(ColorAtom::Rgba8(C))).color, C);
}

#[test]
fn padding_methods_sweep_order_preserved() {
    let style = LayoutStyle::default()
        .padding(8.0)
        .padding_x(16.0)
        .padding_y(24.0)
        .padding_top(32.0)
        .padding_right(40.0)
        .padding_bottom(48.0)
        .padding_left(56.0);
    let padding = style.data().padding;
    assert_eq!(padding.top, LengthPercentage::Px(32.0));
    assert_eq!(padding.right, LengthPercentage::Px(40.0));
    assert_eq!(padding.bottom, LengthPercentage::Px(48.0));
    assert_eq!(padding.left, LengthPercentage::Px(56.0));
}

#[test]
fn sizing_methods_emit_correct_atoms() {
    let style = LayoutStyle::default()
        .width(100.0)
        .height(50.0)
        .width(Dimension::Percent(1.0));
    assert_eq!(style.data().size.width, Dimension::Percent(1.0));
    assert_eq!(style.data().size.height, Dimension::Px(50.0));
}

#[test]
fn overflow_hidden_emits_both_axes() {
    let style = LayoutStyle::default().overflow(Overflow::Hidden);
    assert_eq!(style.data().overflow.width, Overflow::Hidden);
    assert_eq!(style.data().overflow.height, Overflow::Hidden);
}

#[test]
fn rounded_presets_emit_distinct_radii() {
    for (radius, px) in [(Radius::Sm, 2.0), (Radius::Md, 6.0), (Radius::Lg, 8.0)] {
        let rect = rest_rect(pane().bg(A).rounded(radius));
        assert_eq!(rect.border_radius.0[0], px, "{radius:?}");
    }
}

#[test]
fn all_four_state_slots_populate_independently() {
    let node = || {
        pane()
            .bg(REST)
            .hover(|s| s.bg(A))
            .focus(|s| s.bg(B))
            .pressed(|s| s.bg(C))
            .disabled_style(|s| s.bg(D))
    };
    assert_eq!(colors(node(), Drive::Rest), [REST]);
    assert!(colors(node(), Drive::Hover).contains(&A));
    assert!(colors(node(), Drive::Focus).contains(&B));
    assert!(colors(node(), Drive::Press).contains(&C));
}

#[test]
fn state_builder_implements_styled_within_closure() {
    let node = pane()
        .bg(REST)
        .hover(|s| s.bg(Token::SurfaceHover).rounded(Radius::Lg).opacity(1.0));
    let hovered = rects(node, Drive::Hover);
    let color = Theme::default().surface_hover;
    let rect = hovered
        .iter()
        .find(|rect| rect.color == color)
        .expect("hover bg");
    assert_eq!(rect.border_radius.0[0], 8.0);
}

#[test]
fn color_or_token_from_impls_route_correctly() {
    let from_rgba: ColorOrToken = color(1, 2, 3, 4).into();
    let from_token: ColorOrToken = Token::Surface.into();
    let from_atom: ColorOrToken = ColorAtom::Rgba8(color(9, 9, 9, 9)).into();

    assert!(matches!(from_rgba, ColorOrToken::Rgba(_)));
    assert!(matches!(
        from_token,
        ColorOrToken::Token(ColorKey::Token(Token::Surface))
    ));
    assert!(matches!(from_atom, ColorOrToken::Atom(_)));
}

#[test]
fn state_style_builder_can_be_built_standalone() {
    let node = pane().bg(REST).hover(|s: StateStyleBuilder| s.bg(A));
    assert!(colors(node, Drive::Hover).contains(&A));
}

#[test]
fn font_methods_emit_typography_atoms() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        text("Aa")
            .font_size(14.0)
            .font_weight(600)
            .font_family(FontFamily::Mono)
            .line_height_px(20.0),
    );
    let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
    let scene = emit_scene(&cx, root);
    let style = &scene.find_text("Aa").expect("glyph run").run.style;
    assert_eq!(style.font_size, 14.0);
    assert_eq!(style.font_weight, 600);
    assert_eq!(style.font_family, "mono");
    assert_eq!(style.line_height, TextLineHeight::Absolute(20.0));
}

#[test]
fn flex_layout_methods_emit_correct_atoms() {
    let style = LayoutStyle::default()
        .display(Display::Flex)
        .flex_direction(FlexDirection::Row)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::SpaceBetween)
        .gap(8.0)
        .grow(1.0)
        .shrink(0.5);
    let data = style.data();
    assert_eq!(data.display, Display::Flex);
    assert_eq!(data.flex_direction, FlexDirection::Row);
    assert_eq!(data.align_items, Some(AlignItems::Center));
    assert_eq!(data.justify_content, Some(JustifyContent::SpaceBetween));
    assert_eq!(data.gap.width, LengthPercentage::Px(8.0));
    assert_eq!(data.gap.height, LengthPercentage::Px(8.0));
    assert_eq!(data.flex_grow, 1.0);
    assert_eq!(data.flex_shrink, 0.5);
}

#[test]
fn empty_builder_has_empty_style() {
    assert!(rects(pane(), Drive::Rest).is_empty());
    assert!(rects(pane(), Drive::Hover).is_empty());
}

#[test]
fn flex_grow_takes_a_factor_like_every_other_grow() {
    assert_eq!(LayoutStyle::default().grow(2.0).data().flex_grow, 2.0);
    assert_eq!(LayoutStyle::default().grow(1.0).data().flex_grow, 1.0);
}

#[test]
fn trap2_opacity_f32_scales_to_atom() {
    assert_eq!(painted_opacity(pane().bg(A).opacity(0.5)), Some(5_000));
}

#[test]
fn trap2_opacity_clamps_at_boundaries() {
    assert_eq!(painted_opacity(pane().bg(A).opacity(0.0)).unwrap_or(0), 0);
    assert_eq!(painted_opacity(pane().bg(A).opacity(1.0)), Some(10_000));
}
