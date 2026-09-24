use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle};
use crate::layout::snap::CellForm;
use crate::style::NodeStyleExt;

mod canvas;
mod flow;
mod flow_layout;
mod flow_paint;
mod flow_parse;
mod flow_route;
mod pie;
mod rank;
mod sequence;
mod strokes;

#[must_use]
pub fn mermaid(source: impl Into<String>) -> NodeBuilder {
    let source = source.into();
    let lines = draw(&source);
    let col = container()
        .role(Role::Figure)
        .label(diagram_name(&source))
        .description(bound_source(&source).trim_end().to_string())
        .layout(LayoutStyle::flex_column().fill_width())
        .class("font-mono text-sm");
    let form =
        CellForm::<fn(NodeBuilder, &[String]) -> NodeBuilder>::new(strokes::window, text_lines);
    form.pick()(col, &lines)
}

fn text_lines(mut col: NodeBuilder, lines: &[String]) -> NodeBuilder {
    for line in lines {
        col = col.child(text(line.clone()));
    }
    col
}

fn diagram_name(source: &str) -> String {
    let kind = source
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .and_then(|line| line.split_whitespace().next())
        .unwrap_or("diagram");
    format!("Mermaid {kind}")
}

const MAX_SOURCE_LINES: usize = 256;
const MAX_SOURCE_CHARS: usize = 512;
const MAX_OUTPUT_LINES: usize = 512;
const MAX_OUTPUT_CHARS: usize = 1024;

fn bound_source(source: &str) -> String {
    let mut out = String::new();
    for (index, line) in source.lines().enumerate() {
        if index >= MAX_SOURCE_LINES {
            out.push_str("source truncated");
            break;
        }
        out.extend(line.chars().take(MAX_SOURCE_CHARS));
        out.push('\n');
    }
    out
}

fn bound_output(mut lines: Vec<String>) -> Vec<String> {
    lines.truncate(MAX_OUTPUT_LINES);
    for line in &mut lines {
        if line.chars().count() > MAX_OUTPUT_CHARS {
            *line = line.chars().take(MAX_OUTPUT_CHARS).collect();
        }
    }
    lines
}

fn draw(source: &str) -> Vec<String> {
    if crate::widgets::source_too_large(source) {
        return vec![String::from("Source is larger than the 1 MiB limit.")];
    }
    let source = bound_source(source);
    let mut lines = flow::draw(&source)
        .or_else(|| sequence::draw(&source))
        .or_else(|| pie::draw(&source))
        .unwrap_or_else(|| framed(&source));
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    bound_output(lines)
}

fn framed(source: &str) -> Vec<String> {
    let mut lines: Vec<String> = source.lines().map(ToString::to_string).collect();
    if lines.is_empty() {
        lines.push(String::new());
    }
    let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let bar = "─".repeat(width.saturating_add(2));
    let mut out = vec![format!("┌{bar}┐")];
    for line in lines {
        let pad = width.saturating_sub(line.chars().count());
        out.push(format!("│ {line}{} │", " ".repeat(pad)));
    }
    out.push(format!("└{bar}┘"));
    out
}

#[cfg(test)]
#[path = "mermaid_tests.rs"]
mod tests;
