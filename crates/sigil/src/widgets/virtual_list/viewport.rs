//! Viewport-root layout helpers for [`super::virtual_list`].
//!
//! Splits the per-axis size decision and the `apply_viewport_overrides`
//! merge out of `mod.rs` to keep the entry-point file under the 400-line
//! hygiene cap.

use kernel::NodeBuilder;
use layout::prelude::{Dimension, LayoutBuilderExt, LayoutStyle, Overflow};

use super::super::layout_ext::LayoutOverrides;

/// Viewport sizing decision passed to [`apply_viewport_overrides`].
///
/// `fill` takes precedence: when true, both axes are set to
/// [`Dimension::Fill`] so the layout engine inherits the parent pane's
/// resolved content box. Otherwise `height` (always) and `width` (when
/// `Some`) are applied as fixed pixel values.
#[derive(Clone, Copy)]
pub(super) struct ViewportSize {
    pub(super) height: f32,
    pub(super) width: Option<f32>,
    pub(super) fill: bool,
}

/// Apply layout overrides on top of the viewport root's base style.
pub(super) fn apply_viewport_overrides(
    root: NodeBuilder,
    overrides: &LayoutOverrides,
    size: ViewportSize,
) -> NodeBuilder {
    let base = LayoutStyle::block().overflow_y(Overflow::Hidden);
    let base = if size.fill {
        base.width(Dimension::Fill).height(Dimension::Fill)
    } else {
        let with_height = base.height(size.height);
        match size.width {
            Some(w) => with_height.width(w),
            None => with_height,
        }
    };
    root.layout(overrides.clone().apply(base))
}
