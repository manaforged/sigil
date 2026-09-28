use sigil::hal::{LogicalSize, Size};
use sigil::layout::TextMeasure;
use sigil::prelude::TextContent;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FixedMeasure {
    pub width: f32,
    pub height: f32,
}

impl TextMeasure for FixedMeasure {
    fn measure(&self, _content: &TextContent, _space: Size) -> LogicalSize {
        LogicalSize::new(self.width, self.height)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharMeasure {
    pub char_width: f32,
    pub line_height: f32,
}

impl TextMeasure for CharMeasure {
    #[allow(
        clippy::cast_precision_loss,
        reason = "test text lengths are far below f32 precision limits"
    )]
    fn measure(&self, content: &TextContent, _space: Size) -> LogicalSize {
        let text = content.resolve();
        let columns = text.lines().map(|line| line.chars().count()).max();
        let rows = text.lines().count().max(1);
        LogicalSize::new(
            columns.unwrap_or(0) as f32 * self.char_width,
            rows as f32 * self.line_height,
        )
    }
}
