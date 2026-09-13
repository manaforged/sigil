use crate::kernel::{NodeBuilder, container, text};
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle};
use crate::style::NodeStyleExt;

mod canvas;
mod flow;
mod pie;
mod sequence;

#[must_use]
pub fn mermaid(source: impl Into<String>) -> NodeBuilder {
    let lines = draw(&source.into());
    let mut col = container()
        .layout(LayoutStyle::flex_column().fill_width())
        .class("font-mono text-sm");
    for line in lines {
        col = col.child(text(line));
    }
    col
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

#[must_use]
pub fn draw(source: &str) -> Vec<String> {
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

#[must_use]
pub fn diagram(source: &str) -> bool {
    let head = source
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("%%"))
        .and_then(|line| line.split_whitespace().next())
        .unwrap_or_default();
    matches!(
        head,
        "graph" | "flowchart" | "stateDiagram" | "stateDiagram-v2" | "sequenceDiagram" | "pie"
    )
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
