//! Layout setter parity across widget builders.
//!
//! Every widget builder gains uniform `.layout()` / `.width()` /
//! `.height()` / `.grow()` / `.padding()` methods through
//! [`WidgetLayoutExt`]. Call sites can then set a non-default size
//! directly on the widget builder instead of wrapping the widget in a
//! sized `container()`:
//!
//! ```ignore
//! use sigil::prelude::*;
//!
//! // before:
//! container()
//!     .layout(LayoutStyle::block().fixed_size(360.0, 200.0))
//!     .child(virtual_list(items, render));
//!
//! // after:
//! virtual_list(items, render)
//!     .width(360.0)
//!     .height(200.0)
//! ```
//!
//! # Naming
//!
//! The trait is intentionally named `WidgetLayoutExt` (not
//! `LayoutBuilderExt`) to avoid a `pub use` collision in
//! `sigil::prelude` with the pre-existing
//! `sigil_layout::prelude::LayoutBuilderExt`, which adds `.layout()`
//! to `NodeBuilder` / `TextBuilder`. The two traits target disjoint
//! impl sets — node primitives versus widget builders — but Rust
//! cannot disambiguate two `pub use` re-exports under the same name.

use sigil_layout::LayoutStyle;

/// Layout overrides accumulated by [`WidgetLayoutExt`] methods on a
/// widget builder, merged onto the builder's default layout at build
/// time.
///
/// Builders embed a `LayoutOverrides` field and consult it after
/// constructing their internal default `LayoutStyle`. The overrides
/// are applied as: full-override (if any) wins as the base, then
/// width / height / grow / padding partial setters apply on top.
///
/// When no overrides have been recorded, [`apply`](Self::apply)
/// returns `base` unchanged — call sites can call it unconditionally
/// instead of branching on a separate "is anything set?" predicate.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutOverrides {
    full: Option<LayoutStyle>,
    width: Option<f32>,
    height: Option<f32>,
    grow: Option<f32>,
    padding: Option<f32>,
}

impl LayoutOverrides {
    /// Construct an empty override set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            full: None,
            width: None,
            height: None,
            grow: None,
            padding: None,
        }
    }

    /// Replace the full layout style.
    ///
    /// Not `const`: `LayoutStyle` now owns a `Vec` (grid tracks), so
    /// dropping the previous `Option<LayoutStyle>` here is a non-trivial
    /// destructor a `const fn` cannot run.
    pub fn layout(&mut self, style: LayoutStyle) {
        self.full = Some(style);
    }

    /// Replace the width override (logical pixels).
    pub const fn width(&mut self, px: f32) {
        self.width = Some(px);
    }

    /// Replace the height override (logical pixels).
    pub const fn height(&mut self, px: f32) {
        self.height = Some(px);
    }

    /// Replace the flex-grow override.
    pub const fn grow(&mut self, factor: f32) {
        self.grow = Some(factor);
    }

    /// Replace the uniform padding override (logical pixels).
    pub const fn padding(&mut self, px: f32) {
        self.padding = Some(px);
    }

    /// The recorded width override, if any. Lets a widget thread a
    /// caller-set `.width(px)` past its own outer node into nested
    /// children (e.g. `text_field` sizing its inner text block to match
    /// the field) before [`apply`](Self::apply) consumes the overrides.
    #[must_use]
    pub const fn width_hint(&self) -> Option<f32> {
        self.width
    }

    /// The recorded height override, if any. Nested widget surfaces use this
    /// to preserve their frame thickness when callers size the outer control.
    #[must_use]
    pub const fn height_hint(&self) -> Option<f32> {
        self.height
    }

    /// Apply the overrides to `base`, returning the merged
    /// [`LayoutStyle`]. The full-override (if set) replaces `base`;
    /// then `width` / `height` / `grow` / `padding` apply on top.
    ///
    /// Cheap when no overrides are present: `base` is returned
    /// unchanged via `Option::unwrap_or` + the empty partial-setter
    /// branches. Call this unconditionally from builders; there is
    /// no separate "should I apply?" check.
    #[must_use]
    pub fn apply(self, base: LayoutStyle) -> LayoutStyle {
        let mut style = self.full.unwrap_or(base);
        if let Some(width) = self.width {
            style = style.width(width);
        }
        if let Some(height) = self.height {
            style = style.height(height);
        }
        if let Some(grow) = self.grow {
            style = style.grow(grow);
        }
        if let Some(padding) = self.padding {
            style = style.padding(padding);
        }
        style
    }
}

/// Layout setters every widget builder gains uniformly.
///
/// The methods write into the builder's internal [`LayoutOverrides`]
/// accumulator; the builder then merges the overrides onto its
/// computed default layout at build time via
/// [`LayoutOverrides::apply`].
///
/// # Inherent vs trait resolution
///
/// `ButtonBuilder` ships an inherent `width(self, f32)` method which
/// takes precedence over this trait's [`WidgetLayoutExt::width`] per
/// Rust's method-resolution order. Importing this trait alongside
/// the button's inherent method continues to call the inherent
/// method on `ButtonBuilder` and the trait method on every other
/// builder.
pub trait WidgetLayoutExt: Sized {
    /// Internal hook every builder fills in; the default-provided
    /// methods below funnel through this single mutator.
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides;

    /// Replace the widget's default layout style with `style`.
    #[must_use]
    fn layout(mut self, style: LayoutStyle) -> Self {
        LayoutOverrides::layout(self.layout_overrides_mut(), style);
        self
    }

    /// Set a fixed logical-pixel width.
    #[must_use]
    fn width(mut self, px: f32) -> Self {
        LayoutOverrides::width(self.layout_overrides_mut(), px);
        self
    }

    /// Set a fixed logical-pixel height.
    #[must_use]
    fn height(mut self, px: f32) -> Self {
        LayoutOverrides::height(self.layout_overrides_mut(), px);
        self
    }

    /// Set the flex-grow factor.
    #[must_use]
    fn grow(mut self, factor: f32) -> Self {
        LayoutOverrides::grow(self.layout_overrides_mut(), factor);
        self
    }

    /// Set uniform padding in logical pixels. This is the box.
    /// `class("p-4")` is the token spelling; if both are set, this wins.
    #[must_use]
    fn padding(mut self, px: f32) -> Self {
        LayoutOverrides::padding(self.layout_overrides_mut(), px);
        self
    }
}

#[cfg(test)]
#[path = "layout_ext_tests.rs"]
mod tests;
