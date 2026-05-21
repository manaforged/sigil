//! Text measurement hook used by the layout adapter.

use hal::{LogicalSize, Size};
use kernel::TextContent;
use style::TextStyle;

/// Text measurement interface used by `LayoutEngine`.
pub trait TextMeasure {
    /// Measure text content under the given available space.
    fn measure(&self, content: &TextContent, space: Size) -> LogicalSize;

    /// Measure text content with resolved typography.
    fn measure_styled(&self, content: &TextContent, style: &TextStyle, space: Size) -> LogicalSize {
        let _ = style;
        self.measure(content, space)
    }

    /// Called once per frame before layout begins.
    ///
    /// Implementations may use this to clear caches or refresh state.
    fn begin_frame(&self) {}
}
