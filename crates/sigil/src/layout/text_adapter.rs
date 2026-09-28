#![cfg_attr(not(any(test, feature = "kittest")), allow(unreachable_pub))]
use std::borrow::Cow;
use std::rc::Rc;

use crate::hal::{Dim, LogicalSize, ParagraphMetrics, Size};
use crate::kernel::TextContent;
use crate::style::{StyledTextEngine, TextStyle};

use crate::layout::TextMeasure;

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
        crate::profile::count(crate::profile::Counter::TextMeasure);
        let text = resolve_text(source);
        let max_width = match space.width {
            Dim::Fixed(w) => Some(w),
            Dim::MinContent | Dim::MaxContent => None,
        };

        let para = match self.engine.layout_paragraph_styled(&text, max_width, style) {
            Ok(para) => para,
            Err(err) => {
                tracing::warn!(target: "sigil::text", error = %err, "text measure failed; reporting zero size");
                return LogicalSize::default();
            }
        };

        let width = match space.width {
            Dim::Fixed(_) | Dim::MaxContent => para.width(),
            Dim::MinContent => para.min_content_width(),
        };

        LogicalSize::new(width, para.height())
    }

    fn begin_frame(&self) {}

    fn font_generation(&self) -> u64 {
        self.engine.font_generation()
    }
}

fn resolve_text(source: &TextContent) -> Cow<'_, str> {
    match source {
        TextContent::Static(s) => Cow::Borrowed(s),
        TextContent::Reactive(_) | TextContent::Compute(_) => Cow::Owned(source.resolve()),
    }
}
