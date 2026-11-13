#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::float_cmp)]

use crate::scene_support;
use crate::widget_support;

use scene_support::color;
use sigil::events::Event;
use sigil::events::EventRouter;
use sigil::geometry::Constraint;
use sigil::geometry::Point;
use sigil::kernel::{Cx, IntoNode, NodeBuilder, container, text};
use sigil::layout::Space;
use sigil::layout::{
    AlignItems, Display, FlexDirection, JustifyContent, LayoutStyle, Length, Overflow,
};
use sigil::scene::{DrawRect, SceneCommand, emit_scene};
use sigil::style::{
    Color, ColorAtom, FontFamily, Paint, Radius, StateStyleBuilder, States, TextLineHeight, Theme,
    Token,
};
use sigil::widgets::Layout;
use widget_support::compute;

const VIEWPORT: Constraint = Constraint::fixed(200.0, 100.0);
const REST: Color = color(240, 240, 240, 255);
const A: Color = color(200, 20, 20, 255);
const B: Color = color(20, 200, 20, 255);
const C: Color = color(20, 20, 200, 255);
const D: Color = color(90, 90, 90, 255);

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
        Drive::Hover => {
            router.route(&mut cx, &layout, Event::pointer_move(Point::new(8.0, 8.0)));
        }
        Drive::Press => {
            router.route(&mut cx, &layout, Event::pointer_move(Point::new(8.0, 8.0)));
            router.route(&mut cx, &layout, Event::pointer_down(Point::new(8.0, 8.0)));
        }
        Drive::Focus => router.set_focus(&mut cx, Some(root)),
    }
    emit_scene(&cx, root, None)
        .commands
        .into_iter()
        .filter_map(|command| match command {
            SceneCommand::Rect(rect) => Some(rect),
            _ => None,
        })
        .collect()
}

fn colors(build: impl IntoNode, drive: Drive) -> Vec<Color> {
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
    for command in emit_scene(&cx, root, None).commands {
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
        .background(Token::Surface)
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(80.0, 20.0)
                .padding_x(Space::Lg),
        )
        .child(
            container()
                .background(A)
                .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
        );
    let all = rects(node, Drive::Rest);
    assert_eq!(all[0].color, Theme::default().color(Token::Surface));
    let child = all.iter().find(|rect| rect.color == A).expect("child rect");
    assert_eq!(child.rect.origin.x, 16.0);
}

#[test]
fn hover_state_record_overwrites_on_second_call() {
    let hovered = colors(
        pane()
            .background(REST)
            .hover(|s| s.background(A))
            .hover(|s| s.background(B)),
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
    let node = pane()
        .background(REST)
        .hover(|s| s.background(A).radius(Radius::Md));
    let hovered = rects(node, Drive::Hover);
    let rect = hovered
        .iter()
        .find(|rect| rect.color == A)
        .expect("hover bg");
    assert_eq!(rect.border_radius.0[0], 6.0);
}

#[test]
fn focus_slot_records_atoms() {
    let focused = colors(
        pane().background(REST).focus(|s| s.background(A)),
        Drive::Focus,
    );
    assert!(focused.contains(&A), "focus slot must paint: {focused:?}");
}

#[test]
fn pressed_slot_records_atoms() {
    let pressed = colors(
        pane().background(REST).pressed(|s| s.background(A)),
        Drive::Press,
    );
    assert!(pressed.contains(&A), "pressed slot must paint: {pressed:?}");
}

#[test]
fn disabled_state_atoms_emit() {
    let node = || pane().background(REST).disabled_paint(|s| s.background(A));
    assert_eq!(colors(node(), Drive::Rest), [REST]);
    assert!(!colors(node(), Drive::Hover).contains(&A));
}

#[test]
fn bg_accepts_rgba8() {
    assert_eq!(rest_rect(pane().background(A)).color, A);
}

#[test]
fn bg_accepts_theme_token() {
    assert_eq!(
        rest_rect(pane().background(Token::Surface)).color,
        Theme::default().color(Token::Surface)
    );
}

#[test]
fn bg_accepts_color_atom() {
    assert_eq!(rest_rect(pane().background(ColorAtom::Color(C))).color, C);
}

#[test]
fn padding_methods_sweep_order_preserved() {
    let style = LayoutStyle::default()
        .padding(Space::Sm)
        .padding_x(Space::Lg)
        .padding_y(Space::Xl)
        .padding_top(Space::Xxl)
        .padding_right(Space::raw(40.0, "test fixture"))
        .padding_bottom(Space::Xxxl)
        .padding_left(Space::raw(56.0, "test fixture"));
    let padding = sigil::testing::style_data(&style).padding;
    assert_eq!(padding.top, Length::Px(32.0));
    assert_eq!(padding.right, Length::Px(40.0));
    assert_eq!(padding.bottom, Length::Px(48.0));
    assert_eq!(padding.left, Length::Px(56.0));
}

#[test]
fn sizing_methods_emit_correct_atoms() {
    let style = LayoutStyle::default()
        .width(100.0)
        .height(50.0)
        .width(Length::Percent(1.0));
    assert_eq!(
        sigil::testing::style_data(&style).size.width,
        Length::Percent(1.0)
    );
    assert_eq!(
        sigil::testing::style_data(&style).size.height,
        Length::Px(50.0)
    );
}

#[test]
fn overflow_hidden_emits_both_axes() {
    let style = LayoutStyle::default().overflow(Overflow::Hidden);
    assert_eq!(
        sigil::testing::style_data(&style).overflow.width,
        Overflow::Hidden
    );
    assert_eq!(
        sigil::testing::style_data(&style).overflow.height,
        Overflow::Hidden
    );
}

#[test]
fn rounded_presets_emit_distinct_radii() {
    for (radius, px) in [(Radius::Sm, 2.0), (Radius::Md, 6.0), (Radius::Lg, 8.0)] {
        let rect = rest_rect(pane().background(A).radius(radius));
        assert_eq!(rect.border_radius.0[0], px, "{radius:?}");
    }
}

#[test]
fn all_four_state_slots_populate_independently() {
    let node = || {
        pane()
            .background(REST)
            .hover(|s| s.background(A))
            .focus(|s| s.background(B))
            .pressed(|s| s.background(C))
            .disabled_paint(|s| s.background(D))
    };
    assert_eq!(colors(node(), Drive::Rest), [REST]);
    assert!(colors(node(), Drive::Hover).contains(&A));
    assert!(colors(node(), Drive::Focus).contains(&B));
    assert!(colors(node(), Drive::Press).contains(&C));
}

#[test]
fn state_builder_implements_styled_within_closure() {
    let node = pane().background(REST).hover(|s| {
        s.background(Token::SurfaceHover)
            .radius(Radius::Lg)
            .opacity(1.0)
    });
    let hovered = rects(node, Drive::Hover);
    let color = Theme::default().color(Token::SurfaceHover);
    let rect = hovered
        .iter()
        .find(|rect| rect.color == color)
        .expect("hover bg");
    assert_eq!(rect.border_radius.0[0], 8.0);
}

#[test]
fn color_atom_from_impls_route_correctly() {
    let from_rgba: ColorAtom = color(1, 2, 3, 4).into();
    let from_token: ColorAtom = Token::Surface.into();

    assert!(matches!(from_rgba, ColorAtom::Color(_)));
    assert!(matches!(from_token, ColorAtom::Token(Token::Surface)));
}

#[test]
fn state_style_builder_can_be_built_standalone() {
    let node = pane()
        .background(REST)
        .hover(|s: StateStyleBuilder| s.background(A));
    assert!(colors(node, Drive::Hover).contains(&A));
}

#[test]
fn font_methods_emit_typography_atoms() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        text("Aa")
            .font_size(14.0, "test fixture")
            .font_weight(600)
            .font_family(FontFamily::Mono)
            .line_height(20.0),
    );
    let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
    let scene = emit_scene(&cx, root, None);
    let style = scene.find_text("Aa").expect("glyph run").style();
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
        .gap(Space::Sm)
        .grow(1.0)
        .shrink(0.5);
    let data = sigil::testing::style_data(&style);
    assert_eq!(data.display, Display::Flex);
    assert_eq!(data.flex_direction, FlexDirection::Row);
    assert_eq!(data.align_items, Some(AlignItems::Center));
    assert_eq!(data.justify_content, Some(JustifyContent::SpaceBetween));
    assert_eq!(data.gap.width, Length::Px(8.0));
    assert_eq!(data.gap.height, Length::Px(8.0));
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
    assert_eq!(
        sigil::testing::style_data(&LayoutStyle::default().grow(2.0)).flex_grow,
        2.0
    );
    assert_eq!(
        sigil::testing::style_data(&LayoutStyle::default().grow(1.0)).flex_grow,
        1.0
    );
}

#[test]
fn trap2_opacity_f32_scales_to_atom() {
    assert_eq!(
        painted_opacity(pane().background(A).opacity(0.5)),
        Some(5_000)
    );
}

#[test]
fn trap2_opacity_clamps_at_boundaries() {
    assert_eq!(
        painted_opacity(pane().background(A).opacity(0.0)).unwrap_or(0),
        0
    );
    assert_eq!(
        painted_opacity(pane().background(A).opacity(1.0)),
        Some(10_000)
    );
}
