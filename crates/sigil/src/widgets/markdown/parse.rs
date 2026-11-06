use std::ops::Range;

use pulldown_cmark::BlockQuoteKind;

use super::chart_parse::ChartData;
pub(crate) use super::containers::{List, ListItem};
use super::events::located;
pub(crate) use super::table_data::Table;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Block {
    Heading { level: u8, body: Vec<Inline> },
    Paragraph(Vec<Inline>),
    Quote(Vec<Self>),
    Alert { kind: BlockQuoteKind, blocks: Vec<Self> },
    Details { summary: Vec<Inline>, blocks: Vec<Self> },
    List(List),
    Code { lang: String, body: String },
    Math(String),
    Mermaid(String),
    Chart(ChartData),
    Image { alt: String, src: String },
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
    parse_tail(source).0
}

pub(crate) fn parse_tail(source: &str) -> (Vec<Block>, Option<(usize, usize)>) {
    if crate::widgets::source_too_large(source) {
        return (vec![crate::widgets::oversize_block()], None);
    }
    let source = super::latex::delimiters(source);
    let top = located(&source);
    let boundary = cut(&source, &top);
    (top.into_iter().map(|(_, block)| block).collect(), boundary)
}

pub(super) fn nested(source: &str) -> Vec<Block> {
    located(source).into_iter().map(|(_, block)| block).collect()
}

fn cut(source: &str, top: &[(Range<usize>, Block)]) -> Option<(usize, usize)> {
    let [.., (prev, _), (last, _)] = top else {
        return None;
    };
    let gap = source.get(prev.end..last.start)?;
    let head = source.get(..last.start)?.trim_end_matches([' ', '\t']);
    let blank = head
        .strip_suffix('\n')?
        .trim_end_matches([' ', '\t', '\r'])
        .ends_with('\n');
    (blank && gap.trim().is_empty()).then_some((last.start, top.len() - 1))
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
