//! Text measurement hook used by the layout adapter.

use sigil_core::TextSource;
use taffy::prelude::{AvailableSpace, Size};

/// Text measurement interface used by `LayoutEngine`.
pub trait TextMeasure {
    /// Measure a text source under the given available space.
    fn measure(&self, source: &TextSource, space: Size<AvailableSpace>) -> Size<f32>;

    /// Called once per frame before layout begins.
    ///
    /// Implementations may use this to clear caches or refresh state.
    fn begin_frame(&self) {}
}
