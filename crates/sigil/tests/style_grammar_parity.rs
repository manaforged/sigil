#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

use sigil::style::{StyleAtom, parse_class_string};

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
            .any(|a| matches!(a, StyleAtom::BorderThemeRef("accent"))),
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
        ("min-h-4", |a| matches!(a, StyleAtom::MinHeight(_))),
        ("max-h-4", |a| matches!(a, StyleAtom::MaxHeight(_))),
        ("grow", |a| matches!(a, StyleAtom::FlexGrow(1000))),
        ("grow-0", |a| matches!(a, StyleAtom::FlexGrow(0))),
        ("shrink", |a| matches!(a, StyleAtom::FlexShrink(1000))),
        ("shrink-0", |a| matches!(a, StyleAtom::FlexShrink(0))),
        ("grid-cols-2", |a| matches!(a, StyleAtom::GridColumns(2))),
        ("grid-rows-2", |a| matches!(a, StyleAtom::GridRows(2))),
        ("line-through", |a| {
            matches!(
                a,
                StyleAtom::TextDecoration(sigil::style::TextDecoration::LineThrough)
            )
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
