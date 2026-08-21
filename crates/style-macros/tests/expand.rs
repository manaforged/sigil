#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use sigil_style::{ColorAtom, Elevation, Px, StyleAtom, classes};

#[test]
fn expands_static_literal() {
    assert_eq!(
        classes!("p-4 bg-accent"),
        &[
            StyleAtom::PaddingAll(Px(16)),
            StyleAtom::BackgroundThemeRef("accent"),
        ],
    );
}

#[test]
fn expands_match_literal() {
    #[allow(dead_code)]
    enum Size {
        Sm,
        Md,
    }
    let size = Size::Md;

    assert_eq!(
        classes!(
            "bg-accent",
            match size {
                Size::Sm => "h-8 text-sm",
                Size::Md => "h-10 text-base",
            },
        ),
        &[
            StyleAtom::BackgroundThemeRef("accent"),
            StyleAtom::Height(sigil_style::Dimension::Px(Px(40))),
            StyleAtom::FontSize(Px(16)),
        ],
    );
}

#[test]
fn expands_shadow_elevations() {
    assert_eq!(
        classes!("shadow-sm shadow shadow-md shadow-lg shadow-xl"),
        &[
            StyleAtom::Shadow(Elevation::Sm),
            StyleAtom::Shadow(Elevation::Base),
            StyleAtom::Shadow(Elevation::Md),
            StyleAtom::Shadow(Elevation::Lg),
            StyleAtom::Shadow(Elevation::Xl),
        ],
    );
}

#[test]
fn expands_grid_and_text_decoration() {
    assert_eq!(
        classes!("grid-cols-2 line-through"),
        &[
            StyleAtom::GridColumns(2),
            StyleAtom::TextDecoration(sigil_style::TextDecoration::LineThrough),
        ],
    );
}

#[test]
fn concrete_imports_still_work() {
    let color = ColorAtom::ThemeRef("accent");
    assert_eq!(color, ColorAtom::ThemeRef("accent"));
}
