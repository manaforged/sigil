use crate::layout::{TextEngineMeasure, TextMeasure};
use crate::style::TextStyle;
use crate::text::OwnedTextEngine;

pub struct DefaultTextMeasure {
    inner: TextEngineMeasure<OwnedTextEngine>,
}

impl Default for DefaultTextMeasure {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultTextMeasure {
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
        content: &crate::kernel::TextContent,
        space: crate::hal::Size,
    ) -> crate::hal::LogicalSize {
        self.inner.measure(content, space)
    }

    fn measure_styled(
        &self,
        content: &crate::kernel::TextContent,
        style: &TextStyle,
        space: crate::hal::Size,
    ) -> crate::hal::LogicalSize {
        self.inner.measure_styled(content, style, space)
    }

    fn begin_frame(&self) {
        self.inner.begin_frame();
    }
}
