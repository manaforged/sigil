use crate::style::merge::bump_unknown_class_count;
use crate::style::{
    ColorAtom, StyleAtom, Token, merge_classes, merge_warning_count_for_component,
    unknown_class_warning_count,
};

const SURFACE: StyleAtom = StyleAtom::BackgroundColor(ColorAtom::Token(Token::Surface));
const ACCENT: StyleAtom = StyleAtom::BackgroundColor(ColorAtom::Token(Token::Accent));

#[test]
fn merge_last_wins() {
    let merged = merge_classes(&[SURFACE], &[ACCENT], 1);

    assert_eq!(merged.as_slice(), &[ACCENT]);
}

#[test]
fn merge_padding_axis_override_keeps_other_sides() {
    let merged = merge_classes(&[StyleAtom::PaddingAll(16)], &[StyleAtom::PaddingX(24)], 2);

    assert_eq!(
        merged.as_slice(),
        &[
            StyleAtom::PaddingTop(16),
            StyleAtom::PaddingRight(24),
            StyleAtom::PaddingBottom(16),
            StyleAtom::PaddingLeft(24),
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
        StyleAtom::PaddingTop(1),
        StyleAtom::PaddingRight(1),
        StyleAtom::PaddingBottom(1),
        StyleAtom::PaddingLeft(1),
        StyleAtom::MarginTop(1),
        StyleAtom::MarginRight(1),
        StyleAtom::MarginBottom(1),
        StyleAtom::MarginLeft(1),
    ];
    let overrides = [
        StyleAtom::Width(crate::style::atom::ATOM_FULL, 0),
        StyleAtom::Height(crate::style::atom::ATOM_AUTO, 0),
        ACCENT,
        StyleAtom::TextColor(ColorAtom::Token(Token::OnAccent)),
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
