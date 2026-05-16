//! Generic adapter from [`StyledTextEngine`] to [`TextMeasure`].
//!
//! This bridges the HAL text engine trait into the layout-specific
//! measurement hook, keeping `sigil-text` free of any layout dep.

use std::borrow::Cow;
use std::rc::Rc;

use sigil_hal::ParagraphMetrics;
use sigil_kernel::TextContent;
use sigil_style::{StyledTextEngine, TextStyle};

use crate::TextMeasure;

/// Adapts any [`StyledTextEngine`] into a [`TextMeasure`] for use with
/// [`LayoutEngine`](crate::LayoutEngine).
///
/// Uses `Rc` for shared ownership — intentionally `!Send`. Layout
/// and text measurement are main-thread operations in Sigil's
/// single-threaded UI model.
pub struct TextEngineMeasure<E: StyledTextEngine> {
    engine: Rc<E>,
}

impl<E: StyledTextEngine> TextEngineMeasure<E> {
    /// Wrap a shared text engine for use as a layout measurer.
    #[must_use]
    pub const fn new(engine: Rc<E>) -> Self {
        Self { engine }
    }
}

impl<E: StyledTextEngine> TextMeasure for TextEngineMeasure<E> {
    fn measure(
        &self,
        source: &TextContent,
        space: taffy::prelude::Size<taffy::prelude::AvailableSpace>,
    ) -> taffy::prelude::Size<f32> {
        self.measure_styled(source, &TextStyle::default(), space)
    }

    fn measure_styled(
        &self,
        source: &TextContent,
        style: &TextStyle,
        space: taffy::prelude::Size<taffy::prelude::AvailableSpace>,
    ) -> taffy::prelude::Size<f32> {
        use taffy::prelude::{AvailableSpace, Size};

        let text = resolve_text(source);
        let max_width = match space.width {
            AvailableSpace::Definite(w) => Some(w),
            AvailableSpace::MinContent | AvailableSpace::MaxContent => None,
        };

        let Ok(para) = self.engine.layout_paragraph_styled(&text, max_width, style) else {
            return Size::ZERO;
        };

        let width = match space.width {
            AvailableSpace::Definite(_) | AvailableSpace::MaxContent => para.width(),
            AvailableSpace::MinContent => para.min_content_width(),
        };

        Size {
            width,
            height: para.height(),
        }
    }

    fn begin_frame(&self) {
        self.engine.clear_cache();
    }
}

/// Resolve text without allocating for the static case.
fn resolve_text(source: &TextContent) -> Cow<'_, str> {
    match source {
        TextContent::Static(s) => Cow::Borrowed(s),
        TextContent::Reactive(sig) => Cow::Owned(sig.peek()),
    }
}
