#![allow(missing_docs)]
#![allow(clippy::panic, clippy::unwrap_used)]

use allocation_counter as _;
use criterion as _;
use phf as _;
use sigil_core as _;
use sigil_hal as _;
use sigil_reactive as _;
use sigil_style_macros as _;
use smallvec as _;
use strsim as _;

use sigil_style::{AlignKind, DisplayKind, FlexDir, JustifyKind, MillisU16, Style, TransitionId};
use sigil_style::{ColorAtom, Dimension, Px, RenderStyle, Rgba8, StyleAtom, StyleEngine, Theme};
use sigil_style::{NodeStyleExt, StyleBinding};
use sigil_style::{ParseError, parse_class_string};

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
            StyleAtom::BackgroundThemeRef("accent"),
            StyleAtom::TextColor(ColorAtom::ThemeRef("on-accent")),
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
        engine.apply(&[StyleAtom::BackgroundThemeRef("accnet")], &mut render);
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

    engine.apply(&[StyleAtom::BackgroundThemeRef("accnet")], &mut render);

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

#[allow(clippy::too_many_lines)]
#[test]
fn struct_api_covers_every_atom() {
    let color = Rgba8 {
        r: 10,
        g: 20,
        b: 30,
        a: 255,
    };
    let atoms = [
        StyleAtom::PaddingAll(Px(1)),
        StyleAtom::PaddingX(Px(1)),
        StyleAtom::PaddingY(Px(1)),
        StyleAtom::PaddingTop(Px(1)),
        StyleAtom::PaddingRight(Px(1)),
        StyleAtom::PaddingBottom(Px(1)),
        StyleAtom::PaddingLeft(Px(1)),
        StyleAtom::MarginAll(Px(1)),
        StyleAtom::MarginX(Px(1)),
        StyleAtom::MarginY(Px(1)),
        StyleAtom::MarginTop(Px(1)),
        StyleAtom::MarginRight(Px(1)),
        StyleAtom::MarginBottom(Px(1)),
        StyleAtom::MarginLeft(Px(1)),
        StyleAtom::Gap(Px(1)),
        StyleAtom::Width(Dimension::Full),
        StyleAtom::Height(Dimension::Auto),
        StyleAtom::MinWidth(Dimension::Px(Px(1))),
        StyleAtom::MaxWidth(Dimension::Px(Px(1))),
        StyleAtom::Display(DisplayKind::Flex),
        StyleAtom::FlexDirection(FlexDir::Column),
        StyleAtom::AlignItems(AlignKind::Center),
        StyleAtom::JustifyContent(JustifyKind::Between),
        StyleAtom::FlexGrow(1),
        StyleAtom::FlexShrink(1),
        StyleAtom::BackgroundColor(ColorAtom::Rgba8(color)),
        StyleAtom::BackgroundThemeRef("accent"),
        StyleAtom::TextColor(ColorAtom::ThemeRef("on-accent")),
        StyleAtom::TextThemeRef("on-surface"),
        StyleAtom::BorderRadius(Px(1)),
        StyleAtom::Opacity(10_000),
        StyleAtom::FontSize(Px(12)),
        StyleAtom::FontWeight(700),
        StyleAtom::LineHeight(Px(16)),
        StyleAtom::Transition(TransitionId(1)),
        StyleAtom::TransitionDuration(MillisU16(150)),
        StyleAtom::UnknownThemeRef("shadow-xl"),
    ];

    for atom in atoms {
        assert_eq!(style_for_atom(atom).as_atoms(), &[atom]);
    }
}

#[test]
fn struct_api_produces_same_atoms_as_raw_parse() {
    let theme = Theme::default();
    let style = Style::new().padding(8).bg(theme.accent).radius(6);

    assert_eq!(
        style.as_atoms(),
        &[
            StyleAtom::PaddingAll(Px(8)),
            StyleAtom::BackgroundColor(ColorAtom::Rgba8(theme.accent)),
            StyleAtom::BorderRadius(Px(6)),
        ],
    );
}

fn style_for_atom(atom: StyleAtom) -> Style {
    match atom {
        StyleAtom::PaddingAll(px) => Style::new().padding(px.0),
        StyleAtom::PaddingX(px) => Style::new().px(px.0),
        StyleAtom::PaddingY(px) => Style::new().py(px.0),
        StyleAtom::PaddingTop(px) => Style::new().pt(px.0),
        StyleAtom::PaddingRight(px) => Style::new().pr(px.0),
        StyleAtom::PaddingBottom(px) => Style::new().pb(px.0),
        StyleAtom::PaddingLeft(px) => Style::new().pl(px.0),
        StyleAtom::MarginAll(px) => Style::new().margin(px.0),
        StyleAtom::MarginX(px) => Style::new().mx(px.0),
        StyleAtom::MarginY(px) => Style::new().my(px.0),
        StyleAtom::MarginTop(px) => Style::new().mt(px.0),
        StyleAtom::MarginRight(px) => Style::new().mr(px.0),
        StyleAtom::MarginBottom(px) => Style::new().mb(px.0),
        StyleAtom::MarginLeft(px) => Style::new().ml(px.0),
        StyleAtom::Gap(px) => Style::new().gap(px.0),
        StyleAtom::Width(dimension) => Style::new().width(dimension),
        StyleAtom::Height(dimension) => Style::new().height(dimension),
        StyleAtom::MinWidth(dimension) => Style::new().min_width(dimension),
        StyleAtom::MaxWidth(dimension) => Style::new().max_width(dimension),
        StyleAtom::Display(display) => Style::new().display(display),
        StyleAtom::FlexDirection(direction) => Style::new().flex_direction(direction),
        StyleAtom::AlignItems(align) => Style::new().align_items(align),
        StyleAtom::JustifyContent(justify) => Style::new().justify_content(justify),
        StyleAtom::FlexGrow(grow) => Style::new().flex_grow(grow),
        StyleAtom::FlexShrink(shrink) => Style::new().flex_shrink(shrink),
        StyleAtom::BackgroundColor(color) => Style::new().background_color(color),
        StyleAtom::BackgroundThemeRef(token) => Style::new().background_theme(token),
        StyleAtom::TextColor(color) => Style::new().text_color(color),
        StyleAtom::TextThemeRef(token) => Style::new().text_theme(token),
        StyleAtom::BorderRadius(px) => Style::new().radius(px.0),
        StyleAtom::Opacity(opacity) => Style::new().opacity(opacity),
        StyleAtom::FontSize(px) => Style::new().font_size(px.0),
        StyleAtom::FontWeight(weight) => Style::new().font_weight(weight),
        StyleAtom::LineHeight(px) => Style::new().line_height(px.0),
        StyleAtom::Transition(transition) => Style::new().transition(transition),
        StyleAtom::TransitionDuration(duration) => Style::new().transition_duration(duration),
        StyleAtom::UnknownThemeRef(token) => Style::new().unknown_theme_ref(token),
    }
}

#[test]
fn parse_minimal() {
    assert_eq!(
        parse_class_string("p-4 rounded-md bg-accent")
            .unwrap()
            .as_slice(),
        &[
            StyleAtom::PaddingAll(Px(16)),
            StyleAtom::BorderRadius(Px(6)),
            StyleAtom::BackgroundThemeRef("accent"),
        ],
    );
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn parse_grammar_branches() {
    let parsed = parse_class_string(
        "px-2 py-3 pt-1 pr-1 pb-1 pl-1 m-1 mx-2 my-3 mt-1 mr-1 mb-1 ml-1 \
         gap-2 w-4 h-5 w-full h-auto flex block hidden flex-row flex-col \
         items-start items-center items-end items-stretch justify-start justify-center \
         justify-end justify-between justify-around rounded rounded-sm rounded-lg rounded-full \
         text-xs text-sm text-base text-lg text-xl text-on-surface font-normal font-medium \
         font-bold leading-6 opacity-75 border-accent",
    )
    .unwrap();

    assert!(parsed.contains(&StyleAtom::PaddingX(Px(8))));
    assert!(parsed.contains(&StyleAtom::PaddingY(Px(12))));
    assert!(parsed.contains(&StyleAtom::MarginAll(Px(4))));
    assert!(parsed.contains(&StyleAtom::Gap(Px(8))));
    assert!(parsed.contains(&StyleAtom::Width(Dimension::Px(Px(16)))));
    assert!(parsed.contains(&StyleAtom::Height(Dimension::Px(Px(20)))));
    assert!(parsed.contains(&StyleAtom::Width(Dimension::Full)));
    assert!(parsed.contains(&StyleAtom::Height(Dimension::Auto)));
    assert!(parsed.contains(&StyleAtom::Display(DisplayKind::Flex)));
    assert!(parsed.contains(&StyleAtom::Display(DisplayKind::Block)));
    assert!(parsed.contains(&StyleAtom::Display(DisplayKind::None)));
    assert!(parsed.contains(&StyleAtom::FlexDirection(FlexDir::Row)));
    assert!(parsed.contains(&StyleAtom::FlexDirection(FlexDir::Column)));
    assert!(parsed.contains(&StyleAtom::AlignItems(AlignKind::Stretch)));
    assert!(parsed.contains(&StyleAtom::JustifyContent(JustifyKind::Around)));
    assert!(parsed.contains(&StyleAtom::BorderRadius(Px(2))));
    assert!(parsed.contains(&StyleAtom::BorderRadius(Px(8))));
    assert!(parsed.contains(&StyleAtom::BorderRadius(Px(9_999))));
    assert!(parsed.contains(&StyleAtom::FontSize(Px(12))));
    assert!(parsed.contains(&StyleAtom::FontSize(Px(20))));
    assert!(parsed.contains(&StyleAtom::TextThemeRef("on-surface")));
    assert!(parsed.contains(&StyleAtom::FontWeight(400)));
    assert!(parsed.contains(&StyleAtom::FontWeight(500)));
    assert!(parsed.contains(&StyleAtom::FontWeight(700)));
    assert!(parsed.contains(&StyleAtom::LineHeight(Px(24))));
    assert!(parsed.contains(&StyleAtom::Opacity(7_500)));
    assert!(parsed.contains(&StyleAtom::UnknownThemeRef("border-accent")));
}

#[test]
fn parse_rejects_interpolation_attempt() {
    assert!(matches!(
        parse_class_string("bg-${}"),
        Err(ParseError::Unknown { .. }),
    ));
}

#[test]
fn parse_unknown_structural_error() {
    let err = parse_class_string("p-nope").unwrap_err();

    assert!(matches!(err, ParseError::Unknown { .. }));
    assert!(
        err.to_string()
            .contains("Unknown structural token 'p-nope'")
    );
    assert!(err.to_string().contains("nearest:"));
}

#[test]
fn parse_responsive_prefix_not_supported() {
    assert_eq!(
        parse_class_string("sm:p-4"),
        Err(ParseError::ResponsiveNotSupported {
            prefix: "sm".to_owned(),
        }),
    );
}

#[test]
fn parse_pseudo_prefix_not_supported() {
    assert_eq!(
        parse_class_string("hover:bg-accent"),
        Err(ParseError::PseudoNotSupported {
            prefix: "hover".to_owned(),
        }),
    );
}

#[test]
fn class_string_attaches_binding() {
    let mut ptrs = Vec::new();
    for _ in 0..10 {
        let mut cx = sigil_core::Cx::new_root();
        let id = cx.add(sigil_core::container().class("p-4 rounded-md bg-accent"));
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
    let mut cx = sigil_core::Cx::new_root();
    let id = cx.add(sigil_core::text("label").class("text-on-surface"));
    let binding = cx
        .get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .unwrap();

    assert_eq!(
        binding.static_slice(),
        &[StyleAtom::TextThemeRef("on-surface")]
    );
    assert_eq!(binding.dynamic_slice(), &[]);
}

#[test]
fn style_struct_attaches_binding() {
    let mut cx = sigil_core::Cx::new_root();
    let id = cx.add(sigil_core::container().style(Style::new().padding(8)));
    let binding = cx
        .get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .unwrap();

    assert_eq!(binding.static_slice(), &[]);
    assert_eq!(binding.dynamic_slice(), &[StyleAtom::PaddingAll(Px(8))]);
}

#[test]
fn theme_swap_invalidates_binding() {
    let mut cx = sigil_core::Cx::new_root();
    let id = cx.add(sigil_core::container().class("p-4").theme(Theme::default()));
    let node = cx.get(id).unwrap();
    let binding = node.get_extension::<StyleBinding>().unwrap();

    assert!(node.get_extension::<Theme>().is_some());
    assert_eq!(binding.theme_generation(), u64::MAX);
}
