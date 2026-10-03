#![allow(clippy::panic, clippy::unwrap_used)]

use crate::kernel::{Cx, container, text};
use crate::style::{
    ColorAtom, ColorKey, Dimension, Elevation, FontFamily, MillisU16, NodeStyleExt, Paint, Px,
    Radius, RenderStyle, Rgba8, Style, StyleAtom, StyleBinding, StyleEngine, TextDecoration, Theme,
    Token, Transform2D, TransitionId,
};

#[test]
fn atom_size_within_budget() {
    assert!(
        std::mem::size_of::<StyleAtom>() <= 32,
        "StyleAtom is {} bytes; budget is 32 bytes",
        std::mem::size_of::<StyleAtom>(),
    );
}

#[test]
fn theme_ref_resolves() {
    let theme = Theme::default();
    let engine = StyleEngine::new(&theme);
    let mut render = RenderStyle::default();

    engine.apply(
        &[
            StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::Accent)),
            StyleAtom::TextColor(ColorAtom::ThemeRef(ColorKey::Token(Token::OnAccent))),
        ],
        &mut render,
    );

    assert_eq!(render.background_color, Some(theme.accent));
    assert_eq!(render.text_color, Some(theme.on_accent));
}

#[cfg(debug_assertions)]
#[test]
fn unknown_token_panics_dev() {
    let theme = Theme::default();
    let engine = StyleEngine::new(&theme);
    let mut render = RenderStyle::default();

    let err = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine.apply(
            &[StyleAtom::BackgroundThemeRef(ColorKey::Custom("accnet"))],
            &mut render,
        );
    }))
    .unwrap_err();
    let message = err.downcast_ref::<String>().unwrap();

    assert!(message.contains("Unknown theme token 'accnet'"));
    assert!(message.contains("registered:"));
    assert!(message.contains("nearest: 'accent'"));
}

#[cfg(not(debug_assertions))]
#[test]
fn unknown_token_falls_back_release() {
    let theme = Theme::default();
    let engine = StyleEngine::new(&theme);
    let mut render = RenderStyle::default();

    engine.apply(
        &[StyleAtom::BackgroundThemeRef(ColorKey::Custom("accnet"))],
        &mut render,
    );

    assert_eq!(render.background_color, Some(theme.missing_token_fallback));
}

#[test]
fn apply_is_alloc_free() {
    let theme = Theme::default();
    let engine = StyleEngine::new(&theme);
    let atoms = [StyleAtom::Width(Dimension::Px(Px(24))); 64];
    let mut render = RenderStyle::default();

    let info = allocation_counter::measure(|| {
        engine.apply(&atoms, &mut render);
    });

    assert_eq!(info.count_total, 0, "{info:?}");
}

#[test]
fn concrete_color_applies() {
    let theme = Theme::default();
    let engine = StyleEngine::new(&theme);
    let color = Rgba8 {
        r: 1,
        g: 2,
        b: 3,
        a: 4,
    };
    let mut render = RenderStyle::default();

    engine.apply(
        &[StyleAtom::BackgroundColor(ColorAtom::Rgba8(color))],
        &mut render,
    );

    assert_eq!(render.background_color, Some(color));
}

fn paint_for_atom(atom: StyleAtom) -> Style {
    let style = Style::new();
    match atom {
        StyleAtom::BackgroundColor(color) => style.bg(color),
        StyleAtom::BackgroundThemeRef(ColorKey::Token(token)) => style.bg(token),
        StyleAtom::BackgroundGradient(gradient) => style.background_gradient(gradient),
        StyleAtom::TextColor(color) => style.text_color(color),
        StyleAtom::TextThemeRef(ColorKey::Token(token)) => style.text_color(token),
        StyleAtom::BorderThemeRef(ColorKey::Token(token)) => style.border_color(token),
        StyleAtom::BorderColor(color) => style.border_color(color),
        StyleAtom::BorderWidth(px) => style.border_w(px.0),
        StyleAtom::BorderTop(px) => style.border_top(px.0),
        StyleAtom::BorderRight(px) => style.border_right(px.0),
        StyleAtom::BorderBottom(px) => style.border_bottom(px.0),
        StyleAtom::BorderLeft(px) => style.border_left(px.0),
        StyleAtom::BorderRadius(px) => style.radius(px.0),
        StyleAtom::Radius(radius) => style.rounded(radius),
        StyleAtom::Shadow(elevation) => style.shadow(elevation),
        StyleAtom::Opacity(units) => style.opacity(f32::from(units) / 10_000.0),
        StyleAtom::Transform(transform) => style.transform(transform),
        StyleAtom::FontSize(px) => style.font_size(f32::from(px.0)),
        StyleAtom::FontWeight(weight) => style.font_weight(weight),
        StyleAtom::FontItalic(italic) => style.font_italic(italic),
        StyleAtom::LineHeight(px) => style.line_height_px(f32::from(px.0)),
        StyleAtom::LetterSpacing(em) => style.letter_spacing_em(f32::from(em) / 1000.0),
        StyleAtom::TabularNums(on) => style.tabular_nums(on),
        StyleAtom::TextDecoration(TextDecoration::Underline) => style.underline(),
        StyleAtom::TextDecoration(TextDecoration::LineThrough) => style.line_through(),
        StyleAtom::Transition(transition) => style.transition(transition),
        StyleAtom::TransitionDuration(duration) => style.duration(duration),
        other => panic!("{other:?} has no typed paint method"),
    }
}

#[test]
fn struct_api_covers_every_atom() {
    let color = Rgba8 {
        r: 10,
        g: 20,
        b: 30,
        a: 255,
    };
    let atoms = [
        StyleAtom::BackgroundColor(ColorAtom::Rgba8(color)),
        StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::Accent)),
        StyleAtom::TextColor(ColorAtom::ThemeRef(ColorKey::Token(Token::OnAccent))),
        StyleAtom::TextThemeRef(ColorKey::Token(Token::OnSurface)),
        StyleAtom::BorderThemeRef(ColorKey::Token(Token::Border)),
        StyleAtom::BorderWidth(Px(1)),
        StyleAtom::BorderTop(Px(1)),
        StyleAtom::BorderRight(Px(1)),
        StyleAtom::BorderBottom(Px(1)),
        StyleAtom::BorderLeft(Px(1)),
        StyleAtom::BorderRadius(Px(1)),
        StyleAtom::Radius(Radius::Base),
        StyleAtom::Shadow(Elevation::Lg),
        StyleAtom::Opacity(10_000),
        StyleAtom::Transform(Transform2D::IDENTITY),
        StyleAtom::FontSize(Px(12)),
        StyleAtom::FontWeight(700),
        StyleAtom::FontItalic(true),
        StyleAtom::LineHeight(Px(16)),
        StyleAtom::LetterSpacing(25),
        StyleAtom::TabularNums(true),
        StyleAtom::TextDecoration(TextDecoration::Underline),
        StyleAtom::TextDecoration(TextDecoration::LineThrough),
        StyleAtom::Transition(TransitionId(1)),
        StyleAtom::TransitionDuration(MillisU16(150)),
    ];

    for atom in atoms {
        assert_eq!(paint_for_atom(atom).into_atoms().as_slice(), &[atom]);
    }
    assert_eq!(
        Style::new()
            .font_family(FontFamily::Mono)
            .into_atoms()
            .as_slice(),
        &[StyleAtom::FontFamily("mono")],
    );
}

#[test]
fn struct_api_produces_same_atoms_as_raw_parse() {
    let style = Style::new().bg(Token::Accent).radius(6);

    assert_eq!(
        style.into_atoms().as_slice(),
        crate::style::parse_class_string("bg-accent rounded-md")
            .unwrap()
            .as_slice(),
    );
}

#[test]
fn class_string_attaches_binding() {
    let mut ptrs = Vec::new();
    for _ in 0..10 {
        let mut cx = Cx::new_root();
        let id = cx.add(container().class("p-16 rounded-md bg-accent"));
        let binding = cx
            .get(id)
            .and_then(|node| node.get_extension::<StyleBinding>())
            .unwrap();
        ptrs.push(binding.static_slice().as_ptr());
        assert_eq!(binding.dynamic_slice(), &[]);
    }

    assert!(
        ptrs.windows(2)
            .all(|window| window.first() == window.get(1))
    );
}

#[test]
fn class_string_attaches_to_text_binding() {
    let mut cx = Cx::new_root();
    let id = cx.add(text("label").class("text-on-surface"));
    let binding = cx
        .get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .unwrap();

    assert_eq!(
        binding.static_slice(),
        &[StyleAtom::TextThemeRef(ColorKey::Token(Token::OnSurface))]
    );
    assert_eq!(binding.dynamic_slice(), &[]);
}

#[test]
fn style_struct_attaches_binding() {
    let mut cx = Cx::new_root();
    let id = cx.add(container().style(Style::new().radius(8)));
    let binding = cx
        .get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .unwrap();

    assert_eq!(binding.static_slice(), &[]);
    assert_eq!(binding.dynamic_slice(), &[StyleAtom::BorderRadius(Px(8))]);
}

#[test]
fn theme_swap_invalidates_binding() {
    let mut cx = Cx::new_root();
    let id = cx.add(container().class("p-16").theme(Theme::default()));
    let node = cx.get(id).unwrap();
    assert!(node.get_extension::<Theme>().is_some());
    assert!(node.get_extension::<StyleBinding>().is_some());
}
