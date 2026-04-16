//! Shared test stub replacing the removed `DefaultTextMeasure`.

use sigil_core::TextSource;
use sigil_layout::{LayoutEngine, TextMeasure};
use taffy::prelude::{AvailableSpace, Size};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    #[allow(clippy::cast_precision_loss)]
    fn measure(&self, source: &TextSource, _space: Size<AvailableSpace>) -> Size<f32> {
        let chars = match source {
            TextSource::Static(v) => v.chars().count() as f32,
            TextSource::Reactive(_) => 0.0,
        };
        Size {
            width: chars * 8.0,
            height: 16.0,
        }
    }
}

#[allow(clippy::redundant_pub_crate)]
pub(crate) fn stub_engine() -> LayoutEngine {
    LayoutEngine::with_text_measure(Box::new(StubMeasure))
}
