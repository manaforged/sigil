#![allow(clippy::panic, clippy::unwrap_used)]

use std::time::Duration;

use crate::animation::runtime::registry::{config, presets, timing, transition_config};
use crate::animation::{Easing, Transition};
use crate::kernel::{Cx, container, text};
use crate::style::atom::NO_GRADIENT;
use crate::style::{
    Background, Color, ColorAtom, Elevation, FontFamily, Gradient, GradientAxis, GradientShape,
    MillisU16, Paint, Radial, Radius, RenderStyle, Style, StyleAtom, StyleBinding, StyleEngine,
    TextDecoration, Theme, Token, Transform2D, TransitionId, merge_classes,
};
use crate::widgets::Widget;

fn applied(theme: &Theme, atoms: &[StyleAtom]) -> RenderStyle {
    let mut render = RenderStyle::default();
    StyleEngine::new(theme).apply(atoms, &mut render);
    render
}

fn binding(cx: &Cx, id: crate::kernel::NodeId) -> &StyleBinding {
    cx.get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .unwrap()
}

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
    let render = applied(
        &theme,
        &[
            StyleAtom::BackgroundColor(ColorAtom::Token(Token::Accent)),
            StyleAtom::TextColor(ColorAtom::Token(Token::OnAccent)),
        ],
    );

    assert_eq!(render.background_color, Some(theme.accent));
    assert_eq!(render.text_color, Some(theme.on_accent));
}

#[cfg(debug_assertions)]
#[test]
fn unknown_token_panics_dev() {
    let err = std::panic::catch_unwind(|| {
        applied(
            &Theme::default(),
            &[StyleAtom::BackgroundColor(ColorAtom::Custom("accnet"))],
        )
    })
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
    let render = applied(
        &theme,
        &[StyleAtom::BackgroundColor(ColorAtom::Custom("accnet"))],
    );

    assert_eq!(render.background_color, Some(theme.missing_token_fallback));
}

#[test]
fn apply_is_alloc_free() {
    let theme = Theme::default();
    let engine = StyleEngine::new(&theme);
    let atoms = [StyleAtom::Width(crate::style::atom::ATOM_PX, 24); 64];
    let mut render = RenderStyle::default();

    let info = allocation_counter::measure(|| {
        engine.apply(&atoms, &mut render);
    });

    assert_eq!(info.count_total, 0, "{info:?}");
}

#[test]
fn concrete_color_applies() {
    let color = Color {
        r: 1,
        g: 2,
        b: 3,
        a: 4,
    };
    let render = applied(
        &Theme::default(),
        &[StyleAtom::BackgroundColor(ColorAtom::Color(color))],
    );

    assert_eq!(render.background_color, Some(color));
}

fn paint_for_atom(atom: StyleAtom) -> Style {
    let style = Style::new();
    match atom {
        StyleAtom::BackgroundColor(color) => style.background(color),
        StyleAtom::BackgroundGradient(gradient) => style.background(gradient),
        StyleAtom::TextColor(color) => style.text_color(color),
        StyleAtom::BorderColor(color) => style.border_color(color),
        StyleAtom::BorderWidth(px) => style.border_width(px),
        StyleAtom::BorderTop(px) => style.border_top(px),
        StyleAtom::BorderRight(px) => style.border_right(px),
        StyleAtom::BorderBottom(px) => style.border_bottom(px),
        StyleAtom::BorderLeft(px) => style.border_left(px),
        StyleAtom::BorderRadius(px) => style.radius_px(px),
        StyleAtom::Radius(radius) => style.radius(radius),
        StyleAtom::Shadow(elevation) => style.shadow(elevation),
        StyleAtom::Opacity(units) => style.opacity(f32::from(units) / 10_000.0),
        StyleAtom::Transform(transform) => style.transform(transform),
        StyleAtom::FontSize(px) => style.font_size(f32::from(px), ""),
        StyleAtom::FontWeight(weight) => style.font_weight(weight),
        StyleAtom::FontItalic(italic) => style.font_italic(italic),
        StyleAtom::LineHeight(px) => style.line_height_px(f32::from(px)),
        StyleAtom::LetterSpacing(em) => style.letter_spacing_em(f32::from(em) / 1000.0),
        StyleAtom::TabularNums(on) => style.tabular_nums(on),
        StyleAtom::TextDecoration(TextDecoration::Underline) => style.underline(),
        StyleAtom::TextDecoration(TextDecoration::LineThrough) => style.line_through(),
        StyleAtom::Transition(transition) => style.transition(transition),
        StyleAtom::Animation(id) => {
            style.animate(crate::animation::Keyframes::clone(&id.get().unwrap()))
        }
        other => panic!("{other:?} has no typed paint method"),
    }
}

fn lowered(atom: StyleAtom) -> Vec<StyleAtom> {
    match atom {
        StyleAtom::BackgroundColor(_) | StyleAtom::BackgroundGradient(_) => vec![atom, NO_GRADIENT],
        StyleAtom::Transition(id) => {
            let (ms, easing, delay) = timing(id);
            vec![
                atom,
                StyleAtom::TransitionDuration(MillisU16(ms)),
                StyleAtom::TransitionEase(easing),
                StyleAtom::TransitionDelay(MillisU16(delay)),
            ]
        }
        _ => vec![atom],
    }
}

#[test]
fn struct_api_covers_every_atom() {
    let color = Color {
        r: 10,
        g: 20,
        b: 30,
        a: 255,
    };
    let atoms = [
        StyleAtom::BackgroundColor(ColorAtom::Color(color)),
        StyleAtom::BackgroundColor(ColorAtom::Token(Token::Accent)),
        StyleAtom::TextColor(ColorAtom::Token(Token::OnAccent)),
        StyleAtom::TextColor(ColorAtom::Token(Token::OnSurface)),
        StyleAtom::BorderColor(ColorAtom::Token(Token::Border)),
        StyleAtom::BorderWidth(1),
        StyleAtom::BorderTop(1),
        StyleAtom::BorderRight(1),
        StyleAtom::BorderBottom(1),
        StyleAtom::BorderLeft(1),
        StyleAtom::BorderRadius(1),
        StyleAtom::Radius(Radius::Base),
        StyleAtom::Shadow(Elevation::Lg),
        StyleAtom::Opacity(10_000),
        StyleAtom::Transform(Transform2D::IDENTITY),
        StyleAtom::FontSize(12),
        StyleAtom::FontWeight(700),
        StyleAtom::FontItalic(true),
        StyleAtom::LineHeight(16),
        StyleAtom::LetterSpacing(25),
        StyleAtom::TabularNums(true),
        StyleAtom::TextDecoration(TextDecoration::Underline),
        StyleAtom::TextDecoration(TextDecoration::LineThrough),
        StyleAtom::Transition(TransitionId(1)),
    ];

    for atom in atoms {
        assert_eq!(paint_for_atom(atom).into_atoms().as_slice(), lowered(atom));
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
fn a_transition_carries_its_duration() {
    assert_eq!(
        Style::new()
            .transition(Transition::from(TransitionId(1)).duration(Duration::from_millis(300)))
            .into_atoms()
            .as_slice(),
        &[
            StyleAtom::Transition(TransitionId(1)),
            StyleAtom::TransitionDuration(MillisU16(300)),
            StyleAtom::TransitionEase(Easing::IN_OUT),
            StyleAtom::TransitionDelay(MillisU16(0)),
        ],
    );
}

fn gradient() -> Gradient {
    Background::linear(GradientAxis::LeftToRight)
        .start(Token::Accent)
        .end(Token::Surface)
}

#[test]
fn a_later_background_replaces_the_earlier_one() {
    let theme = Theme::default();
    let base = Style::new().background(gradient()).into_atoms();
    let hover = Style::new().background(Token::Surface).into_atoms();
    let chained = Style::new()
        .background(gradient())
        .background(Token::Surface)
        .into_atoms();
    let cascade: Vec<StyleAtom> = base.iter().chain(&hover).copied().collect();
    let merged = merge_classes(&base, &hover, 0);

    for atoms in [chained.as_slice(), cascade.as_slice(), merged.as_slice()] {
        let render = applied(&theme, atoms);
        assert_eq!(render.paint.gradient, None, "{atoms:?}");
        assert_eq!(render.paint.shape(), None, "{atoms:?}");
        assert_eq!(
            render.background_color,
            Some(theme.resolve_color(Token::Surface.into())),
        );
    }
}

#[test]
fn a_later_gradient_keeps_nothing_of_the_earlier_background() {
    let theme = Theme::default();
    let radial = Radial::new();
    let chained = Style::new()
        .background(Token::Surface)
        .background(gradient())
        .background(Background::radial(radial).start(Token::Accent))
        .into_atoms();
    let merged = merge_classes(&[], &chained, 0);

    for atoms in [chained.as_slice(), merged.as_slice()] {
        let render = applied(&theme, atoms);
        let accent = theme.resolve_color(Token::Accent.into());
        assert_eq!(render.background_color, None, "{atoms:?}");
        assert_eq!(render.paint.gradient, Some(GradientShape::Radial(radial)));
        assert_eq!(render.paint.stops, [Some(accent), None, None]);
        assert_eq!(render.paint.positions, [Some(0), Some(50), Some(100)]);
    }
}

#[test]
fn a_later_transition_replaces_the_earlier_one() {
    let slow = Transition::all().duration(Duration::from_secs(1));
    let base = Style::new().transition(slow).into_atoms();
    let hover = Style::new().transition(Transition::new()).into_atoms();
    let chained = Style::new()
        .transition(slow)
        .transition(Transition::new())
        .into_atoms();
    let cascade: Vec<StyleAtom> = base.iter().chain(&hover).copied().collect();
    let merged = merge_classes(&base, &hover, 0);

    for atoms in [chained.as_slice(), cascade.as_slice(), merged.as_slice()] {
        let resolved = transition_config(atoms).map(|(timing, _)| timing);
        assert_eq!(resolved, config(presets::TRANSITION), "{atoms:?}");
    }
}

#[test]
fn struct_api_produces_same_atoms_as_raw_parse() {
    let style = Style::new().background(Token::Accent).radius_px(6);
    let mut classes = crate::style::parse_class_string("bg-accent rounded-md").unwrap();
    classes.insert(1, NO_GRADIENT);

    assert_eq!(style.into_atoms().as_slice(), classes.as_slice());
}

#[test]
fn class_string_attaches_binding() {
    let mut ptrs = Vec::new();
    for _ in 0..10 {
        let mut cx = Cx::new_root();
        let id = cx.add(container().class("p-16 rounded-md bg-accent"));
        let binding = binding(&cx, id);
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
    let binding = binding(&cx, id);

    assert_eq!(
        binding.static_slice(),
        &[StyleAtom::TextColor(ColorAtom::Token(Token::OnSurface))]
    );
    assert_eq!(binding.dynamic_slice(), &[]);
}

#[test]
fn style_struct_attaches_binding() {
    let mut cx = Cx::new_root();
    let id = cx.add(container().style(Style::new().radius_px(8)));
    let binding = binding(&cx, id);

    assert_eq!(binding.static_slice(), &[]);
    assert_eq!(binding.dynamic_slice(), &[StyleAtom::BorderRadius(8)]);
}

#[test]
fn theme_swap_invalidates_binding() {
    let mut cx = Cx::new_root();
    let id = cx.add(container().class("p-16").theme(Theme::default()));
    let node = cx.get(id).unwrap();
    assert!(node.get_extension::<Theme>().is_some());
    assert!(node.get_extension::<StyleBinding>().is_some());
}
