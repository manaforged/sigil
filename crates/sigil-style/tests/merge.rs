#![allow(missing_docs)]
#![allow(clippy::panic)]

use allocation_counter as _;
use criterion as _;
use phf as _;
use sigil_core as _;
use sigil_hal as _;
use sigil_reactive as _;
use sigil_style_macros as _;
use smallvec as _;
use strsim as _;

use sigil_style::{Dimension, Px, StyleAtom, merge_classes, merge_warning_count_for_component};

#[test]
fn merge_last_wins() {
    let merged = merge_classes(
        &[StyleAtom::BackgroundThemeRef("surface")],
        &[StyleAtom::BackgroundThemeRef("accent")],
        1,
    );

    assert_eq!(
        merged.as_slice(),
        &[StyleAtom::BackgroundThemeRef("accent")]
    );
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
        let _merged = merge_classes(
            &[StyleAtom::BackgroundThemeRef("surface")],
            &[StyleAtom::BackgroundThemeRef("accent")],
            component_id,
        );
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
        StyleAtom::BackgroundThemeRef("accent"),
        StyleAtom::TextThemeRef("on-accent"),
    ];

    let info = allocation_counter::measure(|| {
        let _merged = merge_classes(&defaults, &overrides, 88);
    });

    assert_eq!(info.count_total, 0, "{info:?}");
}
