//! Text measurement hook used by the layout adapter.

use sigil_core::TextContent;
use taffy::prelude::{AvailableSpace, Size};

/// Text measurement interface used by `LayoutEngine`.
pub trait TextMeasure {
    /// Measure text content under the given available space.
    fn measure(&self, content: &TextContent, space: Size<AvailableSpace>) -> Size<f32>;

    /// Called once per frame before layout begins.
    ///
    /// Implementations may use this to clear caches or refresh state.
    fn begin_frame(&self) {}
}
