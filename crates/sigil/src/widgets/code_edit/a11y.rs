use crate::grapheme::{byte_at_col, col_at_byte, col_count};
use crate::kernel::NodeBuilder;
use crate::widgets::text_edit::TextEditRuntime;

pub(super) const fn expose(node: NodeBuilder, runtime: &TextEditRuntime) -> NodeBuilder {
    node.text_value(runtime.value)
        .text_cursor(runtime.caret, runtime.selection)
}

pub(super) fn byte_at<S: AsRef<str>>(lines: &[S], (line, col): (usize, usize)) -> usize {
    let before: usize = lines
        .iter()
        .take(line)
        .map(|text| text.as_ref().len() + 1)
        .sum();
    let within = lines
        .get(line)
        .map_or(0, |text| byte_at_col(text.as_ref(), col));
    before + within
}

pub(super) fn point_at<S: AsRef<str>>(lines: &[S], byte: usize) -> (usize, usize) {
    let mut start = 0;
    for (index, line) in lines.iter().enumerate() {
        let line = line.as_ref();
        let end = start + line.len();
        if byte <= end {
            return (index, col_at_byte(line, byte - start));
        }
        start = end + 1;
    }
    (
        lines.len().saturating_sub(1),
        lines.last().map_or(0, |line| col_count(line.as_ref())),
    )
}

pub(super) fn joins_to(lines: &[String], text: &str) -> bool {
    if lines.is_empty() {
        return text.is_empty();
    }
    lines.iter().map(String::as_str).eq(text.split('\n'))
}
