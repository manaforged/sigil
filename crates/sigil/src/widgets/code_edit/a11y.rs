use crate::kernel::NodeBuilder;
use crate::widgets::grapheme::{byte_at_col, col_at_byte, col_count};
use crate::widgets::text_edit::TextEditRuntime;

pub(super) const fn expose(node: NodeBuilder, runtime: &TextEditRuntime) -> NodeBuilder {
    node.text_value(runtime.value)
        .text_cursor(runtime.caret, runtime.selection)
}

pub(super) fn byte_at(lines: &[String], (line, col): (usize, usize)) -> usize {
    let before: usize = lines.iter().take(line).map(|text| text.len() + 1).sum();
    let within = lines.get(line).map_or(0, |text| byte_at_col(text, col));
    before + within
}

pub(super) fn point_at(lines: &[String], byte: usize) -> (usize, usize) {
    let mut start = 0;
    for (index, line) in lines.iter().enumerate() {
        let end = start + line.len();
        if byte <= end {
            let col = col_at_byte(line, byte - start);
            return (index, col);
        }
        start = end + 1;
    }
    (
        lines.len().saturating_sub(1),
        lines.last().map_or(0, |line| col_count(line)),
    )
}
