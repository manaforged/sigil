//! Default text measure used by `sigil::launch`.

use std::rc::Rc;

use sigil_layout::{TextEngineMeasure, TextMeasure};
use sigil_style::TextStyle;
use sigil_text::{ParleyTextEngine, TextConfig};

/// Parley-backed text measurer used by the launch pipeline.
pub struct DefaultTextMeasure {
    inner: TextEngineMeasure<ParleyTextEngine>,
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
        let engine = Rc::new(ParleyTextEngine::new(TextConfig::default()));
        Self {
            inner: TextEngineMeasure::new(engine),
        }
    }
}

impl TextMeasure for DefaultTextMeasure {
    fn measure(
        &self,
        content: &sigil_core::TextContent,
        space: taffy::prelude::Size<taffy::prelude::AvailableSpace>,
    ) -> taffy::prelude::Size<f32> {
        self.inner.measure(content, space)
    }

    fn measure_styled(
        &self,
        content: &sigil_core::TextContent,
        style: &TextStyle,
        space: taffy::prelude::Size<taffy::prelude::AvailableSpace>,
    ) -> taffy::prelude::Size<f32> {
        self.inner.measure_styled(content, style, space)
    }

    fn begin_frame(&self) {
        self.inner.begin_frame();
    }
}
