//! Default text measure used by `sigil::launch`.

use std::rc::Rc;

use layout::{TextEngineMeasure, TextMeasure};
use style::TextStyle;
use text::{OwnedTextEngine, TextConfig};

/// Owned-text-engine-backed measurer used by the launch pipeline.
pub struct DefaultTextMeasure {
    inner: TextEngineMeasure<OwnedTextEngine>,
}

impl Default for DefaultTextMeasure {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultTextMeasure {
    /// Create the default text measurer.
    #[must_use]
    pub fn new() -> Self {
        let engine = Rc::new(OwnedTextEngine::new(TextConfig::default()));
        Self {
            inner: TextEngineMeasure::new(engine),
        }
    }
}

impl TextMeasure for DefaultTextMeasure {
    fn measure(&self, content: &kernel::TextContent, space: hal::Size) -> hal::LogicalSize {
        self.inner.measure(content, space)
    }

    fn measure_styled(
        &self,
        content: &kernel::TextContent,
        style: &TextStyle,
        space: hal::Size,
    ) -> hal::LogicalSize {
        self.inner.measure_styled(content, style, space)
    }

    fn begin_frame(&self) {
        self.inner.begin_frame();
    }
}
