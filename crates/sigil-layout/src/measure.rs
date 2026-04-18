//! Text measurement hook used by the layout adapter.

use sigil_core::TextContent;
use sigil_style::TextStyle;
use taffy::prelude::{AvailableSpace, Size};

/// Text measurement interface used by `LayoutEngine`.
pub trait TextMeasure {
    /// Measure text content under the given available space.
    fn measure(&self, content: &TextContent, space: Size<AvailableSpace>) -> Size<f32>;

    /// Measure text content with resolved typography.
    fn measure_styled(
        &self,
        content: &TextContent,
        style: &TextStyle,
        space: Size<AvailableSpace>,
    ) -> Size<f32> {
        let _ = style;
        self.measure(content, space)
    }

    /// Called once per frame before layout begins.
    ///
    /// Implementations may use this to clear caches or refresh state.
    fn begin_frame(&self) {}
}
