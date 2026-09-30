#![allow(clippy::panic, clippy::unwrap_used)]

use crate::style::parser::ParseError;
use crate::style::{
    AlignKind, ColorKey, Dimension, DisplayKind, FlexDir, FlexWrapKind, JustifyKind, OverflowKind,
    Px, StateSlot, StyleAtom, TextDecoration, Token, parse_class_string, parse_stateful_token,
};

#[test]
fn parse_minimal() {
    assert_eq!(
        parse_class_string("p-16 rounded-md bg-accent")
            .unwrap()
            .as_slice(),
        &[
            StyleAtom::PaddingAll(Px(16)),
            StyleAtom::BorderRadius(Px(6)),
            StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::Accent)),
        ],
    );
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn parse_grammar_branches() {
    let parsed = parse_class_string(
        "px-8 py-12 pt-4 pr-4 pb-4 pl-4 m-4 mx-8 my-12 mt-4 mr-4 mb-4 ml-4 \
         gap-8 w-16 h-20 w-full h-auto flex block hidden flex-row flex-col \
         items-start items-center items-end items-stretch justify-start justify-center \
         justify-end justify-between justify-around rounded rounded-sm rounded-lg rounded-full \
         text-xs text-sm text-base text-lg text-xl text-on-surface font-normal font-medium \
         font-bold italic not-italic leading-24 opacity-75",
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
    assert!(parsed.contains(&StyleAtom::FontSize(Px(14))));
    assert!(parsed.contains(&StyleAtom::FontSize(Px(16))));
    assert!(parsed.contains(&StyleAtom::FontSize(Px(18))));
    assert!(parsed.contains(&StyleAtom::FontSize(Px(20))));
    assert!(parsed.contains(&StyleAtom::TextThemeRef(ColorKey::Token(Token::OnSurface))));
    assert!(parsed.contains(&StyleAtom::FontWeight(400)));
    assert!(parsed.contains(&StyleAtom::FontWeight(500)));
    assert!(parsed.contains(&StyleAtom::FontWeight(700)));
    assert!(parsed.contains(&StyleAtom::FontItalic(true)));
    assert!(parsed.contains(&StyleAtom::FontItalic(false)));
    assert!(parsed.contains(&StyleAtom::LineHeight(Px(24))));
    assert!(parsed.contains(&StyleAtom::Opacity(7_500)));
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
        parse_class_string("sm:p-16"),
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
fn t12_vocabulary_tokens_parse() {
    let parsed = parse_class_string("gap-x-16 gap-y-8 flex-wrap justify-evenly overflow-auto")
        .expect("t12 tokens parse");
    assert!(parsed.contains(&StyleAtom::GapX(Px(16))));
    assert!(parsed.contains(&StyleAtom::GapY(Px(8))));
    assert!(parsed.contains(&StyleAtom::FlexWrap(FlexWrapKind::Wrap)));
    assert!(parsed.contains(&StyleAtom::JustifyContent(JustifyKind::Evenly)));
    assert!(parsed.contains(&StyleAtom::OverflowX(OverflowKind::Auto)));
    assert!(parsed.contains(&StyleAtom::OverflowY(OverflowKind::Auto)));

    let nowrap = parse_class_string("flex-nowrap").expect("flex-nowrap parses");
    assert!(nowrap.contains(&StyleAtom::FlexWrap(FlexWrapKind::NoWrap)));
}

#[test]
fn shadow_family_resolves_to_elevation_atoms() {
    for token in ["shadow", "shadow-sm", "shadow-md", "shadow-lg", "shadow-xl"] {
        let atoms = parse_class_string(token)
            .unwrap_or_else(|e| panic!("'{token}' must parse now, got {e:?}"));
        assert!(
            atoms.iter().any(|a| matches!(a, StyleAtom::Shadow(_))),
            "'{token}' must resolve to a Shadow atom, got {atoms:?}"
        );
    }
}

#[test]
fn border_tokens_resolve_to_width_and_color_atoms() {
    let bare = parse_class_string("border").unwrap();
    assert!(
        bare.iter()
            .any(|a| matches!(a, StyleAtom::BorderWidth(px) if px.0 == 1))
    );
    let width = parse_class_string("border-2").unwrap();
    assert!(
        width
            .iter()
            .any(|a| matches!(a, StyleAtom::BorderWidth(px) if px.0 == 2))
    );
    let colored = parse_class_string("border-accent").unwrap();
    assert!(
        colored
            .iter()
            .any(|a| matches!(a, StyleAtom::BorderThemeRef(ColorKey::Token(Token::Accent)))),
        "border-<token> must resolve as a theme colour reference"
    );
    let left = parse_class_string("border-l-4").unwrap();
    assert!(
        left.iter()
            .any(|a| matches!(a, StyleAtom::BorderLeft(px) if px.0 == 4)),
        "border-l-4 must resolve as a left-edge width"
    );
    let axis = parse_class_string("border-x").unwrap();
    assert_eq!(axis.len(), 2, "border-x sets left and right");
}

#[test]
fn overflow_shorthand_expands_to_both_axes() {
    let both_axes = parse_class_string("overflow-hidden").unwrap();
    assert_eq!(both_axes.len(), 2, "overflow-* must set both axes");
    assert!(
        both_axes
            .iter()
            .any(|a| matches!(a, StyleAtom::OverflowX(_)))
    );
    assert!(
        both_axes
            .iter()
            .any(|a| matches!(a, StyleAtom::OverflowY(_)))
    );
    let single_axis = parse_class_string("overflow-y-scroll").unwrap();
    assert_eq!(single_axis.len(), 1);
    assert!(matches!(single_axis.first(), Some(StyleAtom::OverflowY(_))));
}

type AtomCheck = fn(&StyleAtom) -> bool;

#[test]
fn new_parity_tokens_resolve_to_expected_atoms() {
    let cases: [(&str, AtomCheck); 9] = [
        ("min-h-16", |a| matches!(a, StyleAtom::MinHeight(_))),
        ("max-h-16", |a| matches!(a, StyleAtom::MaxHeight(_))),
        ("grow", |a| matches!(a, StyleAtom::FlexGrow(1000))),
        ("grow-0", |a| matches!(a, StyleAtom::FlexGrow(0))),
        ("shrink", |a| matches!(a, StyleAtom::FlexShrink(1000))),
        ("shrink-0", |a| matches!(a, StyleAtom::FlexShrink(0))),
        ("grid-cols-2", |a| matches!(a, StyleAtom::GridColumns(2))),
        ("grid-rows-2", |a| matches!(a, StyleAtom::GridRows(2))),
        ("line-through", |a| {
            matches!(a, StyleAtom::TextDecoration(TextDecoration::LineThrough))
        }),
    ];
    for (token, check) in cases {
        let atoms = parse_class_string(token).unwrap();
        assert!(
            atoms.iter().any(check),
            "'{token}' did not resolve to the expected atom: {atoms:?}"
        );
    }
}

fn route(input: &str) -> Vec<(Option<StateSlot>, StyleAtom)> {
    input
        .split(' ')
        .flat_map(|token| {
            let (slot, atoms) = parse_stateful_token(token).unwrap();
            atoms.into_iter().map(move |atom| (slot, atom))
        })
        .collect()
}

fn slot_atoms(
    routed: &[(Option<StateSlot>, StyleAtom)],
    slot: Option<StateSlot>,
) -> Vec<StyleAtom> {
    routed
        .iter()
        .filter(|(s, _)| *s == slot)
        .map(|(_, atom)| *atom)
        .collect()
}

#[test]
fn class_routes_state_prefixes_to_overlay_slots() {
    let s = route(
        "bg-surface hover:bg-surface-hover active:bg-surface-pressed focus:rounded-md disabled:opacity-50",
    );

    assert_eq!(
        slot_atoms(&s, None),
        [StyleAtom::BackgroundThemeRef(ColorKey::Token(
            Token::Surface
        ))]
    );
    assert_eq!(
        slot_atoms(&s, Some(StateSlot::Hovered)),
        [StyleAtom::BackgroundThemeRef(ColorKey::Token(
            Token::SurfaceHover
        ))]
    );
    assert_eq!(
        slot_atoms(&s, Some(StateSlot::Pressed)),
        [StyleAtom::BackgroundThemeRef(ColorKey::Token(
            Token::SurfacePressed
        ))]
    );
    assert_eq!(
        slot_atoms(&s, Some(StateSlot::Focused)),
        [StyleAtom::BorderRadius(Px(6))]
    );
    assert_eq!(
        slot_atoms(&s, Some(StateSlot::Disabled)),
        [StyleAtom::Opacity(5_000)]
    );
}

#[test]
fn class_accumulates_slot() {
    let s = route("hover:bg-accent hover:rounded-md px-16");

    assert_eq!(
        slot_atoms(&s, Some(StateSlot::Hovered)),
        [
            StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::Accent)),
            StyleAtom::BorderRadius(Px(6)),
        ]
    );
    assert_eq!(slot_atoms(&s, None), [StyleAtom::PaddingX(Px(16))]);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "invalid class")]
fn unknown_class_panics_in_debug() {
    let _ = crate::style::parse_classes("bogus:x px-16");
}
