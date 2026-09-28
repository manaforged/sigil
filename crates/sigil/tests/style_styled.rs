#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]

use sigil::kernel::container;
use sigil::style::{
    ColorAtom, ColorOrToken, Dimension, Fill, Flex, Geometry, OverflowKind, Rgba8, Sizing, Spacing,
    StateStyleBuilder, StateVariants, StyleAtom, Styleable, Typography, Visual,
};
use sigil::style::{ColorKey, Token};

fn styled() -> sigil::style::NodeStyledBuilder {
    container().styled()
}

#[test]
fn default_impl_chains_correctly() {
    let b = styled().bg("surface").px(16);
    let atoms = b.style().as_atoms();
    assert!(
        atoms.contains(&StyleAtom::BackgroundThemeRef(ColorKey::Token(
            Token::Surface
        )))
    );
    assert!(atoms.contains(&StyleAtom::PaddingX(sigil::style::Px(16))));
}

#[test]
fn hover_state_record_overwrites_on_second_call() {
    let b = styled().hover(|s| s.bg("a")).hover(|s| s.bg("b"));
    let hover = b.style().hover_atoms();
    assert_eq!(hover.len(), 1, "hover slot must be last-call-wins");
    assert_eq!(
        hover[0],
        StyleAtom::BackgroundThemeRef(ColorKey::Custom("b"))
    );
}

#[test]
fn hover_atoms_accumulate_within_closure() {
    let b = styled().hover(|s| s.bg("a").px(8));
    let hover = b.style().hover_atoms();
    assert_eq!(hover.len(), 2);
    assert_eq!(
        hover[0],
        StyleAtom::BackgroundThemeRef(ColorKey::Custom("a"))
    );
    assert_eq!(hover[1], StyleAtom::PaddingX(sigil::style::Px(8)));
}

#[test]
fn focus_slot_records_atoms() {
    let b = styled().focus(|s| s.bg("focus-ring"));
    let focus = b.style().focus_atoms();
    assert_eq!(focus.len(), 1);
    assert_eq!(
        focus[0],
        StyleAtom::BackgroundThemeRef(ColorKey::Custom("focus-ring"))
    );
}

#[test]
fn pressed_slot_records_atoms() {
    let b = styled().pressed(|s| s.bg("pressed"));
    let pressed = b.style().pressed_atoms();
    assert_eq!(pressed.len(), 1);
    assert_eq!(
        pressed[0],
        StyleAtom::BackgroundThemeRef(ColorKey::Custom("pressed"))
    );
}

#[test]
fn disabled_state_atoms_emit() {
    let b = styled().disabled_style(|s| s.opacity(0.5));
    let disabled = b.style().disabled_atoms();
    assert_eq!(disabled.len(), 1);
    assert_eq!(disabled[0], StyleAtom::Opacity(5_000));
}

#[test]
fn bg_accepts_rgba8() {
    let red = Rgba8 {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    let b = styled().bg(red);
    let atoms = b.style().as_atoms();
    assert_eq!(atoms.len(), 1);
    assert_eq!(atoms[0], StyleAtom::BackgroundColor(ColorAtom::Rgba8(red)));
}

#[test]
fn bg_accepts_theme_token() {
    let b = styled().bg("surface");
    let atoms = b.style().as_atoms();
    assert_eq!(atoms.len(), 1);
    assert_eq!(
        atoms[0],
        StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::Surface))
    );
}

#[test]
fn bg_accepts_color_atom() {
    let blue = Rgba8 {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };
    let atom = ColorAtom::Rgba8(blue);
    let b = styled().bg(atom);
    let atoms = b.style().as_atoms();
    assert_eq!(atoms.len(), 1);
    assert_eq!(atoms[0], StyleAtom::BackgroundColor(atom));
}

#[test]
fn padding_methods_sweep_order_preserved() {
    let b = styled()
        .padding(8)
        .px(16)
        .py(24)
        .pt(32)
        .pr(40)
        .pb(48)
        .pl(56);
    let atoms = b.style().as_atoms();
    assert_eq!(atoms.len(), 7);
    assert_eq!(atoms[0], StyleAtom::PaddingAll(sigil::style::Px(8)));
    assert_eq!(atoms[1], StyleAtom::PaddingX(sigil::style::Px(16)));
    assert_eq!(atoms[2], StyleAtom::PaddingY(sigil::style::Px(24)));
    assert_eq!(atoms[3], StyleAtom::PaddingTop(sigil::style::Px(32)));
    assert_eq!(atoms[4], StyleAtom::PaddingRight(sigil::style::Px(40)));
    assert_eq!(atoms[5], StyleAtom::PaddingBottom(sigil::style::Px(48)));
    assert_eq!(atoms[6], StyleAtom::PaddingLeft(sigil::style::Px(56)));
}

#[test]
fn sizing_methods_emit_correct_atoms() {
    let b = styled()
        .w(Dimension::Px(sigil::style::Px(100)))
        .h(Dimension::Px(sigil::style::Px(50)))
        .w(Dimension::Full);
    let atoms = b.style().as_atoms();
    assert_eq!(atoms.len(), 3);
    assert_eq!(
        atoms[0],
        StyleAtom::Width(Dimension::Px(sigil::style::Px(100)))
    );
    assert_eq!(
        atoms[1],
        StyleAtom::Height(Dimension::Px(sigil::style::Px(50)))
    );
    assert_eq!(atoms[2], StyleAtom::Width(Dimension::Full));
}

#[test]
fn overflow_hidden_emits_both_axes() {
    let b = styled().overflow_hidden();
    let atoms = b.style().as_atoms();
    assert_eq!(atoms.len(), 2);
    assert_eq!(atoms[0], StyleAtom::OverflowX(OverflowKind::Hidden));
    assert_eq!(atoms[1], StyleAtom::OverflowY(OverflowKind::Hidden));
}

#[test]
fn rounded_presets_emit_distinct_radii() {
    let sm = styled().rounded(2);
    let md = styled().rounded(6);
    let lg = styled().rounded(10);
    assert_eq!(
        sm.style().as_atoms()[0],
        StyleAtom::BorderRadius(sigil::style::Px(2))
    );
    assert_eq!(
        md.style().as_atoms()[0],
        StyleAtom::BorderRadius(sigil::style::Px(6))
    );
    assert_eq!(
        lg.style().as_atoms()[0],
        StyleAtom::BorderRadius(sigil::style::Px(10))
    );
}

#[test]
fn all_four_state_slots_populate_independently() {
    let b = styled()
        .hover(|s| s.bg("h"))
        .focus(|s| s.bg("f"))
        .pressed(|s| s.bg("p"))
        .disabled_style(|s| s.opacity(0.5));
    let style = b.style();
    assert_eq!(style.hover_atoms().len(), 1);
    assert_eq!(style.focus_atoms().len(), 1);
    assert_eq!(style.pressed_atoms().len(), 1);
    assert_eq!(style.disabled_atoms().len(), 1);
    assert!(style.has_state_overlays());
}

#[test]
fn state_builder_implements_styled_within_closure() {
    let b = styled().hover(|s| {
        s.bg("surface-hover")
            .text_color("on-surface")
            .px(12)
            .opacity(0.9)
    });
    let hover = b.style().hover_atoms();
    assert_eq!(hover.len(), 4);
    assert_eq!(
        hover[0],
        StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::SurfaceHover))
    );
    assert_eq!(
        hover[1],
        StyleAtom::TextThemeRef(ColorKey::Token(Token::OnSurface))
    );
    assert_eq!(hover[2], StyleAtom::PaddingX(sigil::style::Px(12)));
    assert_eq!(hover[3], StyleAtom::Opacity(9_000));
}

#[test]
fn color_or_token_from_impls_route_correctly() {
    let from_rgba: ColorOrToken = Rgba8 {
        r: 1,
        g: 2,
        b: 3,
        a: 4,
    }
    .into();
    let from_token: ColorOrToken = "surface".into();
    let from_atom: ColorOrToken = ColorAtom::Rgba8(Rgba8 {
        r: 9,
        g: 9,
        b: 9,
        a: 9,
    })
    .into();

    assert!(matches!(from_rgba, ColorOrToken::Rgba(_)));
    assert!(matches!(
        from_token,
        ColorOrToken::Token(ColorKey::Token(Token::Surface))
    ));
    assert!(matches!(from_atom, ColorOrToken::Atom(_)));
}

#[test]
fn state_style_builder_can_be_built_standalone() {
    let outer = styled().hover(|s: StateStyleBuilder| s.bg("h"));
    let hover = outer.style().hover_atoms();
    assert_eq!(hover.len(), 1);
    assert_eq!(
        hover[0],
        StyleAtom::BackgroundThemeRef(ColorKey::Custom("h"))
    );
}

#[test]
fn font_methods_emit_typography_atoms() {
    let b = styled()
        .font_size(14)
        .font_weight(600)
        .font_family("mono")
        .line_height(20);
    let atoms = b.style().as_atoms();
    assert_eq!(atoms.len(), 4);
    assert_eq!(atoms[0], StyleAtom::FontSize(sigil::style::Px(14)));
    assert_eq!(atoms[1], StyleAtom::FontWeight(600));
    assert_eq!(atoms[2], StyleAtom::FontFamily("mono"));
    assert_eq!(atoms[3], StyleAtom::LineHeight(sigil::style::Px(20)));
}

#[test]
fn flex_layout_methods_emit_correct_atoms() {
    let b = styled()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(8)
        .flex_grow(1000)
        .shrink(500);
    let atoms = b.style().as_atoms();
    assert_eq!(atoms.len(), 7);
    assert_eq!(
        atoms[0],
        StyleAtom::Display(sigil::style::DisplayKind::Flex)
    );
    assert_eq!(
        atoms[1],
        StyleAtom::FlexDirection(sigil::style::FlexDir::Row)
    );
    assert_eq!(
        atoms[2],
        StyleAtom::AlignItems(sigil::style::AlignKind::Center)
    );
    assert_eq!(
        atoms[3],
        StyleAtom::JustifyContent(sigil::style::JustifyKind::Between)
    );
    assert_eq!(atoms[4], StyleAtom::Gap(sigil::style::Px(8)));
    assert_eq!(atoms[5], StyleAtom::FlexGrow(1000));
    assert_eq!(atoms[6], StyleAtom::FlexShrink(500));
}

#[test]
fn empty_builder_has_empty_style() {
    let b = styled();
    let style = b.style();
    assert!(style.is_empty());
    assert_eq!(style.as_atoms().len(), 0);
    assert!(!style.has_state_overlays());
}

#[test]
fn trap1_flex_grow_writes_atom_and_grow_is_gone() {
    let b = styled().flex_grow(2);
    let atoms = b.style().as_atoms();
    assert_eq!(atoms.len(), 1);
    assert_eq!(atoms[0], StyleAtom::FlexGrow(2));

    let b2 = styled().flex_grow(1000);
    assert_eq!(b2.style().as_atoms()[0], StyleAtom::FlexGrow(1000));
}

#[test]
fn trap2_opacity_f32_scales_to_atom() {
    let b = styled().opacity(0.5);
    let atoms = b.style().as_atoms();
    assert_eq!(atoms.len(), 1);
    assert_eq!(atoms[0], StyleAtom::Opacity(5_000));
}

#[test]
fn trap2_opacity_clamps_at_boundaries() {
    let zero = styled().opacity(0.0);
    assert_eq!(zero.style().as_atoms()[0], StyleAtom::Opacity(0));

    let one = styled().opacity(1.0);
    assert_eq!(one.style().as_atoms()[0], StyleAtom::Opacity(10_000));
}

#[test]
fn class_routes_state_prefixes_to_overlay_slots() {
    let s = sigil::style::Style::new()
        .class("bg-surface hover:bg-surface-hover active:bg-surface-pressed focus:rounded-md disabled:opacity-50");

    assert_eq!(
        s.as_atoms(),
        [StyleAtom::BackgroundThemeRef(ColorKey::Token(
            Token::Surface
        ))]
    );
    assert_eq!(
        s.hover_atoms(),
        [StyleAtom::BackgroundThemeRef(ColorKey::Token(
            Token::SurfaceHover
        ))]
    );
    assert_eq!(
        s.pressed_atoms(),
        [StyleAtom::BackgroundThemeRef(ColorKey::Token(
            Token::SurfacePressed
        ))]
    );
    assert_eq!(
        s.focus_atoms(),
        [StyleAtom::BorderRadius(sigil::style::Px(6))]
    );
    assert_eq!(s.disabled_atoms(), [StyleAtom::Opacity(5_000)]);
}

#[test]
fn class_accumulates_slot() {
    let s = sigil::style::Style::new().class("hover:bg-accent hover:rounded-md px-16");

    assert_eq!(
        s.hover_atoms(),
        [
            StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::Accent)),
            StyleAtom::BorderRadius(sigil::style::Px(6)),
        ]
    );
    assert_eq!(s.as_atoms(), [StyleAtom::PaddingX(sigil::style::Px(16))]);
}

#[test]
#[should_panic(expected = "invalid class")]
fn unknown_class_panics_in_debug() {
    let _ = sigil::style::Style::new().class("bogus:x px-16");
}
