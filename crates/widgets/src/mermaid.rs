use sigil_kernel::{NodeBuilder, container, text};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_style::NodeStyleExt;

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

#[must_use]
pub fn draw(source: &str) -> Vec<String> {
    let Some(edges) = flow_edges(source) else {
        return framed(source);
    };
    if edges.is_empty() {
        return framed(source);
    }
    chain_or_fan(&edges)
}

fn flow_edges(source: &str) -> Option<Vec<(String, String)>> {
    let mut edges = Vec::new();
    let mut kind = false;
    for raw in source.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if is_flow_header(line) {
            kind = true;
            continue;
        }
        if let Some(edge) = parse_edge(line) {
            kind = true;
            edges.push(edge);
            continue;
        }
        if !kind {
            return None;
        }
        return None;
    }
    kind.then_some(edges)
}

fn is_flow_header(line: &str) -> bool {
    let head = line.split_whitespace().next().unwrap_or("");
    matches!(head, "graph" | "flowchart")
}

fn parse_edge(line: &str) -> Option<(String, String)> {
    let (left, right) = line.split_once("-->")?;
    Some((node_name(left.trim()), node_name(right.trim())))
}

fn node_name(token: &str) -> String {
    if let Some(inner) = token.strip_suffix(']').and_then(|s| s.split_once('[')) {
        let label = inner.1.trim();
        if !label.is_empty() {
            return label.to_string();
        }
    }
    token
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect()
}

fn chain_or_fan(edges: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (from, to) in edges {
        if seen.insert(from.clone()) {
            push_box(&mut out, from);
        }
        out.push("  │".to_string());
        if seen.insert(to.clone()) {
            push_box(&mut out, to);
        }
    }
    out
}

fn push_box(out: &mut Vec<String>, label: &str) {
    let inner = format!(" {label} ");
    let width = inner.chars().count();
    let bar = "─".repeat(width);
    out.push(format!("┌{bar}┐"));
    out.push(format!("│{inner}│"));
    out.push(format!("└{bar}┘"));
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
