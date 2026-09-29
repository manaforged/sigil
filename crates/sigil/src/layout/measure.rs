use crate::geometry::{LogicalSize, Size};
use crate::kernel::TextContent;
use crate::style::TextStyle;

pub trait TextMeasure {
    fn measure(&self, content: &TextContent, space: Size) -> LogicalSize;

    fn measure_styled(&self, content: &TextContent, style: &TextStyle, space: Size) -> LogicalSize {
        let _ = style;
        self.measure(content, space)
    }

    fn begin_frame(&self) {}

    fn font_generation(&self) -> u64 {
        0
    }
}
