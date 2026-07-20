//! DR-056 D3 grammar/atom parity gate.
//!
//! `style-macros/grammar/structural.toml` is the single source of truth
//! for the utility-token catalog. Every token it lists must parse
//! through the runtime parser into a real atom (never
//! `UnknownThemeRef`), and the CSS-shaped families with no paint path
//! (`border-*`, `shadow-*`) must be rejected loudly, not silently
//! dropped. The macro side of the same contract is pinned by
//! `style-macros/tests/expand.rs` + trybuild compile-fail cases.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]

use allocation_counter as _;
use criterion as _;
use phf as _;
use sigil as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_reactive as _;
use sigil_style_macros as _;
use smallvec as _;
use strsim as _;
use tracing as _;

use std::fs;
use std::path::PathBuf;

use sigil_style::{ParseError, StyleAtom, parse_class_string};

fn grammar_tokens() -> Vec<String> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../style-macros/grammar/structural.toml");
    let raw = fs::read_to_string(path).expect("read structural.toml");
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() || line.starts_with('[') {
                return None;
            }
            let (token, _category) = line.split_once('=')?;
            Some(token.trim().trim_matches('"').to_owned())
        })
        .collect()
}

#[test]
fn every_grammar_token_parses_to_a_real_atom() {
    let tokens = grammar_tokens();
    assert!(
        tokens.len() >= 70,
        "grammar list suspiciously short ({}) — toml parse broke?",
        tokens.len()
    );
    for token in &tokens {
        let atoms = parse_class_string(token)
            .unwrap_or_else(|err| panic!("grammar token '{token}' failed to parse: {err}"));
        assert!(
            !atoms.is_empty(),
            "grammar token '{token}' parsed to zero atoms"
        );
        for atom in &atoms {
            assert!(
                !matches!(atom, StyleAtom::UnknownThemeRef(_)),
                "grammar token '{token}' resolves to UnknownThemeRef — a silent no-op \
                 (DR-056 D2: every accepted token must have an observable effect)"
            );
        }
    }
}

#[test]
fn shadow_family_is_rejected_not_silenced() {
    for token in ["shadow-xl", "shadow-sm"] {
        match parse_class_string(token) {
            Err(ParseError::NotYetRenderable { token: rejected }) => {
                assert_eq!(rejected, token);
            }
            other => panic!(
                "'{token}' must reject as NotYetRenderable until a paint path exists, got {other:?}"
            ),
        }
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
    let cases: [(&str, AtomCheck); 6] = [
        ("min-h-4", |a| matches!(a, StyleAtom::MinHeight(_))),
        ("max-h-4", |a| matches!(a, StyleAtom::MaxHeight(_))),
        ("grow", |a| matches!(a, StyleAtom::FlexGrow(1000))),
        ("grow-0", |a| matches!(a, StyleAtom::FlexGrow(0))),
        ("shrink", |a| matches!(a, StyleAtom::FlexShrink(1000))),
        ("shrink-0", |a| matches!(a, StyleAtom::FlexShrink(0))),
    ];
    for (token, check) in cases {
        let atoms = parse_class_string(token).unwrap();
        assert!(
            atoms.iter().any(check),
            "'{token}' did not resolve to the expected atom: {atoms:?}"
        );
    }
}
