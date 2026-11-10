use std::ops::Range;

use pulldown_cmark::BlockQuoteKind;

use super::chart_parse::ChartData;
pub(crate) use super::containers::{List, ListItem};
use super::events::{located, scan};
use super::refs::{Refs, Report};
pub(crate) use super::table_data::Table;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Block {
    Heading {
        level: u8,
        body: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    Quote(Vec<Self>),
    Alert {
        kind: BlockQuoteKind,
        blocks: Vec<Self>,
    },
    Details {
        summary: Vec<Inline>,
        blocks: Vec<Self>,
    },
    List(List),
    Code {
        lang: String,
        body: String,
    },
    Math(String),
    Mermaid(String),
    VegaLite(String),
    Graphviz(String),
    Chart(ChartData),
    Image {
        alt: String,
        src: String,
    },
    Table(Table),
    Footnotes(Vec<Note>),
    Html(String),
    Rule,
}

impl Eq for Block {}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Note {
    pub(crate) label: String,
    pub(crate) blocks: Vec<Block>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Inline {
    Text(String),
    Strong(Vec<Self>),
    Em(Vec<Self>),
    Strike(Vec<Self>),
    Code(String),
    Math(String),
    Note(String),
    Image {
        alt: String,
        src: String,
    },
    Link {
        href: String,
        body: Vec<Self>,
        label_is_href: bool,
    },
}

impl Inline {
    pub(crate) fn plain(body: &[Self]) -> String {
        let mut out = String::new();
        for inline in body {
            match inline {
                Self::Text(text) | Self::Code(text) | Self::Math(text) | Self::Note(text) => {
                    out.push_str(text);
                }
                Self::Image { alt, .. } => out.push_str(alt),
                Self::Strong(inner)
                | Self::Em(inner)
                | Self::Strike(inner)
                | Self::Link { body: inner, .. } => {
                    out.push_str(&Self::plain(inner));
                }
            }
        }
        out
    }

    pub(crate) fn push_text(out: &mut Vec<Self>, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(Self::Text(last)) = out.last_mut() {
            last.push_str(text);
        } else {
            out.push(Self::Text(text.to_string()));
        }
    }

    pub(crate) fn finish(mut out: Vec<Self>) -> Vec<Self> {
        if out.is_empty() {
            out.push(Self::Text(String::new()));
        }
        out
    }
}

#[cfg(test)]
pub(crate) fn parse_blocks(source: &str) -> Vec<Block> {
    parse_tail(source, &Refs::default()).blocks
}

pub(super) struct Tail {
    pub(super) blocks: Vec<Block>,
    pub(super) cut: Option<(usize, usize)>,
    pub(super) report: Report,
}

pub(super) fn parse_tail(source: &str, refs: &Refs) -> Tail {
    if crate::widgets::source_too_large(source) {
        return Tail {
            blocks: vec![crate::widgets::oversize_block()],
            cut: None,
            report: Report::default(),
        };
    }
    let source = super::latex::delimiters(source);
    let (top, report) = scan(&source, refs);
    let cut = cut(&source, &top);
    let blocks = top.into_iter().map(|(_, block)| block).collect();
    Tail {
        blocks,
        cut,
        report,
    }
}

pub(super) fn nested(source: &str) -> Vec<Block> {
    located(source)
        .into_iter()
        .map(|(_, block)| block)
        .collect()
}

fn cut(source: &str, top: &[(Range<usize>, Block)]) -> Option<(usize, usize)> {
    let last_at = top.len().checked_sub(1)?;
    let at = top
        .iter()
        .position(|(_, block)| matches!(block, Block::Footnotes(_)))
        .map_or(last_at, |note| note.min(last_at));
    let (prev, block) = top.get(at.checked_sub(1)?)?;
    let (last, _) = top.get(at)?;
    if continues(source, prev, block) {
        return None;
    }
    let gap = source.get(prev.end..last.start)?;
    let head = source.get(..last.start)?.trim_end_matches([' ', '\t']);
    let blank = head
        .strip_suffix('\n')?
        .trim_end_matches([' ', '\t', '\r'])
        .ends_with('\n');
    (blank && gap.trim().is_empty()).then_some((last.start, at))
}

fn continues(source: &str, range: &Range<usize>, block: &Block) -> bool {
    match block {
        Block::List(_) | Block::Footnotes(_) => true,
        Block::Code { .. } => !source.get(range.clone()).is_some_and(|code| {
            let code = code.trim_start();
            code.starts_with("```") || code.starts_with("~~~")
        }),
        _ => false,
    }
}

pub(crate) fn parse_inlines(source: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    for block in nested(&super::latex::delimiters(source)) {
        let body = match block {
            Block::Paragraph(body) | Block::Heading { body, .. } => body,
            Block::Html(html) => vec![Inline::Text(html.trim_end().to_string())],
            _ => return vec![Inline::Text(source.to_string())],
        };
        if !out.is_empty() {
            Inline::push_text(&mut out, "\n");
        }
        for inline in body {
            match inline {
                Inline::Text(text) => Inline::push_text(&mut out, &text),
                other => out.push(other),
            }
        }
    }
    Inline::finish(out)
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
