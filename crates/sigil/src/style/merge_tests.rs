use crate::style::merge::bump_unknown_class_count;
use crate::style::{
    ColorKey, Dimension, Px, StyleAtom, Token, merge_classes, merge_warning_count_for_component,
    unknown_class_warning_count,
};

const SURFACE: StyleAtom = StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::Surface));
const ACCENT: StyleAtom = StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::Accent));

#[test]
fn merge_last_wins() {
    let merged = merge_classes(&[SURFACE], &[ACCENT], 1);

    assert_eq!(merged.as_slice(), &[ACCENT]);
}

#[test]
fn merge_padding_axis_override_keeps_other_sides() {
    let merged = merge_classes(
        &[StyleAtom::PaddingAll(Px(16))],
        &[StyleAtom::PaddingX(Px(24))],
        2,
    );

    assert_eq!(
        merged.as_slice(),
        &[
            StyleAtom::PaddingTop(Px(16)),
            StyleAtom::PaddingRight(Px(24)),
            StyleAtom::PaddingBottom(Px(16)),
            StyleAtom::PaddingLeft(Px(24)),
        ],
    );
}

#[test]
fn merge_warns_once() {
    let component_id = 77;
    for _ in 0..1_000 {
        let _merged = merge_classes(&[SURFACE], &[ACCENT], component_id);
    }

    assert_eq!(merge_warning_count_for_component(component_id), 1);
}

#[test]
fn merge_is_alloc_free_below_32() {
    let defaults = [
        StyleAtom::PaddingTop(Px(1)),
        StyleAtom::PaddingRight(Px(1)),
        StyleAtom::PaddingBottom(Px(1)),
        StyleAtom::PaddingLeft(Px(1)),
        StyleAtom::MarginTop(Px(1)),
        StyleAtom::MarginRight(Px(1)),
        StyleAtom::MarginBottom(Px(1)),
        StyleAtom::MarginLeft(Px(1)),
    ];
    let overrides = [
        StyleAtom::Width(Dimension::Full),
        StyleAtom::Height(Dimension::Auto),
        ACCENT,
        StyleAtom::TextThemeRef(ColorKey::Token(Token::OnAccent)),
    ];

    let info = allocation_counter::measure(|| {
        let _merged = merge_classes(&defaults, &overrides, 88);
    });

    assert_eq!(info.count_total, 0, "{info:?}");
}

#[test]
fn unknown_class_counter_increments() {
    let before = unknown_class_warning_count();
    bump_unknown_class_count();
    bump_unknown_class_count();
    let after = unknown_class_warning_count();
    assert_eq!(after - before, 2);
}
