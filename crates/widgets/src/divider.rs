//! Thin visual rule for header/section breaks.
//!
//! Two variants are exposed:
//!
//! * [`horizontal`] — full-width 1px-tall hairline that sits between
//!   stacked blocks (header to body, body to status bar).
//! * [`vertical`] — full-height 1px-wide hairline that separates side-by-side
//!   panes inside a `flex_row`.
//!
//! Both render as a `container().hittable(false)` styled with the
//! `border` theme token, which gives the right tone on every theme in
//! `crates/style/src/theme.rs`.
//!
//! ## Accessibility — purely decorative
//!
//! Earlier revisions set `Role::Splitter` on both variants. ARIA's
//! `splitter` is a focusable, interactive resize handle; a passive 1px
//! hairline is the wrong fit. `accesskit` 0.24 does not expose a
//! dedicated `Separator` variant — its `Splitter` is the closest
//! equivalent and platform-specifically maps to "Separator" on
//! Windows / atspi but "Splitter" on macOS, so the inconsistency
//! would surface only on macOS.
//!
//! Instead, we mark the divider [`hittable(false)`](sigil_kernel::NodeBuilder::hittable),
//! which routes through the kernel's a11y projection as
//! `accesskit::Node::set_hidden`. The hairline is then omitted from
//! every assistive-tech tree — the correct semantic for a purely
//! decorative graphic. Pointer hit-testing is also disabled, which is
//! desired for a 1px rule no user wants to click.
//!
//! See FRICTION.md (scratchpad) — the original hand-rolled workaround
//! was a `flex_row` + layout-gap, which left no visible rule between
//! stacked blocks.

use sigil_kernel::{NodeBuilder, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_style::{NodeStyleExt, StyleAtom};

/// Thickness in logical pixels for both axes.
const HAIRLINE_PX: f32 = 1.0;

static RULE_FILL: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("border")];

/// Horizontal hairline divider — fills the parent's inline axis,
/// `HAIRLINE_PX` tall on the block axis.
///
/// Use between stacked sections (header above body, body above status bar).
/// Purely decorative — hidden from accessibility trees and not
/// pointer-hittable.
#[must_use]
pub fn horizontal() -> NodeBuilder {
    container()
        .hittable(false)
        .layout(LayoutStyle::block().fill_width().height(HAIRLINE_PX))
        .class(RULE_FILL)
}

/// Vertical hairline divider — `HAIRLINE_PX` wide, fills the parent's
/// block axis.
///
/// Use between side-by-side panes inside a `flex_row` layout. Purely
/// decorative — hidden from accessibility trees and not
/// pointer-hittable.
#[must_use]
pub fn vertical() -> NodeBuilder {
    container()
        .hittable(false)
        .layout(LayoutStyle::block().fill_height().width(HAIRLINE_PX))
        .class(RULE_FILL)
}

#[cfg(test)]
#[path = "divider_tests.rs"]
mod tests;
