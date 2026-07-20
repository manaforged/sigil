//! DS1 scale-token end-to-end: parse `bg-<role>-<step>`,
//! `text-<role>-<step>`, the semantic aliases (`bg-surface-subtle`,
//! `text-muted`, `border-strong`), and verify each resolves to the
//! expected concrete color through the live `Theme` resolution path.

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
// Integration tests intentionally keep assertions direct and use
// indexing for fixed test topology. See the hygiene framework.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use sigil_reactive as _;

use sigil_style::scale::{
    amber_light, cyan_light, green_light, iris_dark, iris_light, ruby_light, slate_dark,
    slate_light,
};
use sigil_style::{ColorAtom, Rgba8, StyleAtom, Theme, parse_class_string};

fn resolved_bg(theme: &Theme, class: &str) -> Rgba8 {
    let atoms = parse_class_string(class).expect("parse");
    let StyleAtom::BackgroundThemeRef(token) = atoms[0] else {
        panic!(
            "expected BackgroundThemeRef from {class}, got {:?}",
            atoms[0]
        );
    };
    let atom = ColorAtom::ThemeRef(token);
    let engine = sigil_style::StyleEngine::new(theme);
    engine.resolve_color(atom)
}

fn resolved_text(theme: &Theme, class: &str) -> Rgba8 {
    let atoms = parse_class_string(class).expect("parse");
    let StyleAtom::TextThemeRef(token) = atoms[0] else {
        panic!("expected TextThemeRef from {class}, got {:?}", atoms[0]);
    };
    let atom = ColorAtom::ThemeRef(token);
    let engine = sigil_style::StyleEngine::new(theme);
    engine.resolve_color(atom)
}

#[test]
fn role_step_tokens_resolve_to_scale_steps() {
    let theme = Theme::default();
    // Each `<role>-<step>` token routes through `Theme::scale(role).step(n)`.
    assert_eq!(resolved_bg(&theme, "bg-gray-2"), slate_light().step(2));
    assert_eq!(resolved_bg(&theme, "bg-gray-12"), slate_light().step(12));
    assert_eq!(resolved_bg(&theme, "bg-accent-9"), iris_light().step(9));
    assert_eq!(resolved_bg(&theme, "bg-success-3"), green_light().step(3));
    assert_eq!(resolved_bg(&theme, "bg-warn-9"), amber_light().step(9));
    assert_eq!(resolved_bg(&theme, "bg-error-9"), ruby_light().step(9));
    assert_eq!(resolved_bg(&theme, "bg-info-3"), cyan_light().step(3));

    assert_eq!(
        resolved_text(&theme, "text-success-11"),
        green_light().step(11)
    );
    assert_eq!(
        resolved_text(&theme, "text-error-11"),
        ruby_light().step(11)
    );
}

#[test]
fn semantic_aliases_resolve_to_scale_steps() {
    let theme = Theme::default();
    // Aliases follow the parser-strip convention: `bg-X` looks up `X`,
    // `text-X` looks up `X`. So the alias names live without the
    // leading `bg-` / `text-`.
    assert_eq!(
        resolved_bg(&theme, "bg-surface-subtle"),
        slate_light().step(3)
    );
    assert_eq!(
        resolved_text(&theme, "text-on-surface-muted"),
        slate_light().step(11)
    );
    assert_eq!(
        resolved_text(&theme, "text-on-surface"),
        slate_light().step(12)
    );
}

#[test]
fn dark_theme_flips_every_scale_and_semantic_alias() {
    let theme = Theme::dark();
    // Scales: step values flip from light to dark Radix.
    assert_eq!(resolved_bg(&theme, "bg-gray-2"), slate_dark().step(2));
    assert_eq!(resolved_bg(&theme, "bg-gray-12"), slate_dark().step(12));
    assert_eq!(resolved_bg(&theme, "bg-accent-9"), iris_dark().step(9));

    // Flat-field shim populated from dark scales.
    assert_eq!(theme.surface, slate_dark().step(2));
    assert_eq!(theme.on_surface, slate_dark().step(12));
    assert_eq!(theme.accent, iris_dark().step(9));
    assert_eq!(theme.border, slate_dark().step(7));

    // Semantic aliases keep their step references — values flip.
    assert_eq!(
        resolved_bg(&theme, "bg-surface-subtle"),
        slate_dark().step(3)
    );
    assert_eq!(
        resolved_text(&theme, "text-on-surface-muted"),
        slate_dark().step(11)
    );

    // Dark differs from light in the same slot.
    assert_ne!(theme.surface, slate_light().step(2));
    assert_ne!(theme.on_surface, slate_light().step(12));
}

#[test]
fn customizing_accent_scale_propagates_through_role_tokens() {
    // Swapping the accent scale must change every `accent-N` resolution
    // — proves the "hella customizable" story.
    let theme = Theme::default().with_accent_scale(ruby_light());
    assert_eq!(resolved_bg(&theme, "bg-accent-9"), ruby_light().step(9));
    assert_eq!(resolved_bg(&theme, "bg-accent-3"), ruby_light().step(3));
    // The flat `theme.accent` mirror also updates.
    assert_eq!(theme.accent, ruby_light().step(9));
}
