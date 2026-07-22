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
//! [`sigil::prelude`] with the pre-existing
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

    /// Set uniform padding in logical pixels.
    #[must_use]
    fn padding(mut self, px: f32) -> Self {
        LayoutOverrides::padding(self.layout_overrides_mut(), px);
        self
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    missing_docs
)]
mod tests {
    use super::*;
    use sigil_layout::LayoutStyle;
    use sigil_layout::prelude::Dimension;

    // ---- LayoutOverrides unit tests -----------------------------------------

    #[test]
    fn empty_overrides_are_pass_through() {
        let base = LayoutStyle::flex_row().fixed_size(10.0, 20.0);
        let result = LayoutOverrides::new().apply(base.clone());
        assert_eq!(result, base);
    }

    #[test]
    fn width_height_override_replaces_dimensions() {
        let mut overrides = LayoutOverrides::new();
        overrides.width(100.0);
        overrides.height(50.0);
        let base = LayoutStyle::flex_row().fixed_size(10.0, 20.0);
        let result = overrides.apply(base);
        let expected = LayoutStyle::flex_row()
            .fixed_size(10.0, 20.0)
            .width(100.0)
            .height(50.0);
        assert_eq!(result, expected);
    }

    #[test]
    fn full_layout_replaces_base() {
        let mut overrides = LayoutOverrides::new();
        let replacement = LayoutStyle::block().fixed_size(7.0, 9.0);
        overrides.layout(replacement.clone());
        let base = LayoutStyle::flex_row().fixed_size(10.0, 20.0);
        let result = overrides.apply(base);
        assert_eq!(result, replacement);
    }

    #[test]
    fn full_layout_then_partials_compose() {
        let mut overrides = LayoutOverrides::new();
        let replacement = LayoutStyle::block().fixed_size(7.0, 9.0);
        overrides.layout(replacement.clone());
        overrides.width(123.0);
        let base = LayoutStyle::flex_row().fixed_size(10.0, 20.0);
        let result = overrides.apply(base);
        let expected = replacement.width(123.0);
        assert_eq!(result, expected);
    }

    // ---- per-builder smoke tests --------------------------------------------
    // Each builder gains .width()/.height() from WidgetLayoutExt.
    // We verify the override reaches the root node's LayoutStyle extension.

    fn root_layout_style(cx: &sigil_kernel::Cx, root: sigil_kernel::NodeId) -> LayoutStyle {
        cx.get(root)
            .expect("root node exists")
            .get_extension::<LayoutStyle>()
            .expect("root has LayoutStyle")
            .clone()
    }

    #[test]
    fn button_height_override_reaches_root_node() {
        use crate::widgets::button::button;
        sigil_reactive::Runtime::scope("layout-ext-button-smoke", || {
            let mut cx = sigil_kernel::Cx::new_root();
            // .width(100.0) calls ButtonBuilder's inherent setter (intrinsic width).
            // .height(50.0) goes through WidgetLayoutExt into layout_overrides.
            let root = cx.add(button("OK").width(100.0).height(50.0));
            let style = root_layout_style(&cx, root);
            assert_eq!(style.data().size.width, Dimension::Px(100.0));
            assert_eq!(style.data().size.height, Dimension::Px(50.0));
        });
    }

    #[test]
    fn toggle_width_height_override_reaches_root_node() {
        use crate::widgets::toggle::toggle;
        sigil_reactive::Runtime::scope("layout-ext-toggle-smoke", || {
            let mut cx = sigil_kernel::Cx::new_root();
            let on = sigil_reactive::signal(false);
            let root = cx.add(toggle(on).width(100.0).height(50.0));
            let style = root_layout_style(&cx, root);
            assert_eq!(style.data().size.width, Dimension::Px(100.0));
            assert_eq!(style.data().size.height, Dimension::Px(50.0));
        });
    }

    #[test]
    fn checkbox_width_height_override_reaches_root_node() {
        use crate::widgets::checkbox::checkbox;
        sigil_reactive::Runtime::scope("layout-ext-checkbox-smoke", || {
            let mut cx = sigil_kernel::Cx::new_root();
            let checked = sigil_reactive::signal(false);
            let root = cx.add(checkbox(checked).width(100.0).height(50.0));
            let style = root_layout_style(&cx, root);
            assert_eq!(style.data().size.width, Dimension::Px(100.0));
            assert_eq!(style.data().size.height, Dimension::Px(50.0));
        });
    }

    #[test]
    fn text_field_width_height_override_reaches_root_node() {
        use crate::widgets::text_field::text_field;
        sigil_reactive::Runtime::scope("layout-ext-text-field-smoke", || {
            let mut cx = sigil_kernel::Cx::new_root();
            let value = sigil_reactive::signal(String::new());
            let root = cx.add(text_field(value).width(100.0).height(50.0));
            let style = root_layout_style(&cx, root);
            assert_eq!(style.data().size.width, Dimension::Px(100.0));
            assert_eq!(style.data().size.height, Dimension::Px(50.0));
        });
    }

    #[test]
    fn virtual_list_width_height_override_reaches_root_node() {
        use crate::widgets::virtual_list::virtual_list;
        sigil_reactive::Runtime::scope("layout-ext-virtual-list-smoke", || {
            let mut cx = sigil_kernel::Cx::new_root();
            let items: Vec<u32> = (0..10).collect();
            let root = cx.add(
                virtual_list(items, |_, _| sigil_kernel::container())
                    .width(100.0)
                    .height(50.0),
            );
            let style = root_layout_style(&cx, root);
            assert_eq!(style.data().size.width, Dimension::Px(100.0));
            assert_eq!(style.data().size.height, Dimension::Px(50.0));
        });
    }

    #[test]
    fn splitter_width_height_override_reaches_root_node() {
        use crate::widgets::splitter::{SplitterOrientation, splitter};
        sigil_reactive::Runtime::scope("layout-ext-splitter-smoke", || {
            let mut cx = sigil_kernel::Cx::new_root();
            let root = cx.add(
                splitter(SplitterOrientation::Horizontal)
                    .child(sigil_kernel::container(), 200.0)
                    .child(sigil_kernel::container(), 200.0)
                    .width(100.0)
                    .height(50.0),
            );
            let style = root_layout_style(&cx, root);
            assert_eq!(style.data().size.width, Dimension::Px(100.0));
            assert_eq!(style.data().size.height, Dimension::Px(50.0));
        });
    }

    #[test]
    fn tabs_width_height_override_reaches_root_node() {
        use crate::widgets::tabs::tabs;
        sigil_reactive::Runtime::scope("layout-ext-tabs-smoke", || {
            let mut cx = sigil_kernel::Cx::new_root();
            let active = sigil_reactive::signal(0_usize);
            let labels = vec![String::from("A"), String::from("B")];
            let root = cx.add(tabs(active, labels).width(100.0).height(50.0));
            let style = root_layout_style(&cx, root);
            assert_eq!(style.data().size.width, Dimension::Px(100.0));
            assert_eq!(style.data().size.height, Dimension::Px(50.0));
        });
    }

    #[test]
    fn modal_width_height_override_reaches_root_node() {
        use crate::widgets::modal::modal;
        sigil_reactive::Runtime::scope("layout-ext-modal-smoke", || {
            let mut cx = sigil_kernel::Cx::new_root();
            let show = sigil_reactive::signal(true);
            let root = cx.add(modal().show(show).width(100.0).height(50.0));
            let style = root_layout_style(&cx, root);
            assert_eq!(style.data().size.width, Dimension::Px(100.0));
            assert_eq!(style.data().size.height, Dimension::Px(50.0));
        });
    }
}
