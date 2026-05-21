//! Shared test stub replacing the removed `DefaultTextMeasure`.

use hal::{LogicalSize, Size};
use kernel::TextContent;
use layout::{LayoutEngine, TextMeasure};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    #[allow(clippy::cast_precision_loss)]
    fn measure(&self, source: &TextContent, _space: Size) -> LogicalSize {
        let chars = match source {
            TextContent::Static(v) => v.chars().count() as f32,
            TextContent::Reactive(_) => 0.0,
        };
        LogicalSize::new(chars * 8.0, 16.0)
    }
}

#[allow(clippy::redundant_pub_crate)]
pub(crate) fn stub_engine() -> LayoutEngine {
    LayoutEngine::with_text_measure(Box::new(StubMeasure))
}
