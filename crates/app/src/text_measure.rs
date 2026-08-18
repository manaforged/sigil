//! Default text measure used by `sigil::launch`.

use sigil_layout::{TextEngineMeasure, TextMeasure};
use sigil_style::TextStyle;
use sigil_text::OwnedTextEngine;

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
        Self {
            inner: TextEngineMeasure::new(OwnedTextEngine::thread()),
        }
    }
}

impl TextMeasure for DefaultTextMeasure {
    fn measure(
        &self,
        content: &sigil_kernel::TextContent,
        space: sigil_hal::Size,
    ) -> sigil_hal::LogicalSize {
        self.inner.measure(content, space)
    }

    fn measure_styled(
        &self,
        content: &sigil_kernel::TextContent,
        style: &TextStyle,
        space: sigil_hal::Size,
    ) -> sigil_hal::LogicalSize {
        self.inner.measure_styled(content, style, space)
    }

    fn begin_frame(&self) {
        self.inner.begin_frame();
    }
}
