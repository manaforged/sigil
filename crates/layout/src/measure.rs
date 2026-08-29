use sigil_hal::{LogicalSize, Size};
use sigil_kernel::TextContent;
use sigil_style::TextStyle;

pub trait TextMeasure {
    fn measure(&self, content: &TextContent, space: Size) -> LogicalSize;

    fn measure_styled(&self, content: &TextContent, style: &TextStyle, space: Size) -> LogicalSize {
        let _ = style;
        self.measure(content, space)
    }

    fn begin_frame(&self) {}
}
