use std::borrow::Cow;
use std::rc::Rc;

use sigil_hal::{Dim, LogicalSize, ParagraphMetrics, Size};
use sigil_kernel::TextContent;
use sigil_style::{StyledTextEngine, TextStyle};

use crate::TextMeasure;

pub struct TextEngineMeasure<E: StyledTextEngine> {
    engine: Rc<E>,
}

impl<E: StyledTextEngine> TextEngineMeasure<E> {
    #[must_use]
    pub const fn new(engine: Rc<E>) -> Self {
        Self { engine }
    }
}

impl<E: StyledTextEngine> TextMeasure for TextEngineMeasure<E> {
    fn measure(&self, source: &TextContent, space: Size) -> LogicalSize {
        self.measure_styled(source, &TextStyle::default(), space)
    }

    fn measure_styled(&self, source: &TextContent, style: &TextStyle, space: Size) -> LogicalSize {
        sigil_profile::count("text.measure");
        let text = resolve_text(source);
        let max_width = match space.width {
            Dim::Fixed(w) => Some(w),
            Dim::MinContent | Dim::MaxContent => None,
        };

        let Ok(para) = self.engine.layout_paragraph_styled(&text, max_width, style) else {
            return LogicalSize::default();
        };

        let width = match space.width {
            Dim::Fixed(_) | Dim::MaxContent => para.width(),
            Dim::MinContent => para.min_content_width(),
        };

        LogicalSize::new(width, para.height())
    }

    fn begin_frame(&self) {}
}

fn resolve_text(source: &TextContent) -> Cow<'_, str> {
    match source {
        TextContent::Static(s) => Cow::Borrowed(s),
        TextContent::Reactive(_) | TextContent::Compute(_) => Cow::Owned(source.resolve()),
    }
}
