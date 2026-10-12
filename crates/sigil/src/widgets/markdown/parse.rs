use super::chart_parse::{ChartData, fence_block};
pub(crate) use super::containers::{List, ListItem};
use super::containers::{
    heading, image_line, list_marker, rule, take_list, take_quote, take_table,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Block {
    Heading { level: u8, text: String },
    Paragraph(String),
    Quote(Vec<Self>),
    List(List),
    Code { lang: String, body: String },
    Mermaid(String),
    Chart(ChartData),
    Image { alt: String, src: String },
    Table(Vec<Vec<String>>),
    Rule,
}

impl Eq for Block {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Span {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    Link { text: String, href: String },
}

#[cfg(test)]
pub(crate) use super::inline::parse_inlines;

const MAX_NESTING: usize = 16;

#[derive(Default)]
struct Blocks {
    out: Vec<Block>,
    para: String,
    fence: Option<(String, String)>,
    depth: usize,
}

#[cfg(test)]
pub(crate) fn parse_blocks(source: &str) -> Vec<Block> {
    parse_tail(source).0
}

pub(crate) fn parse_tail(source: &str) -> (Vec<Block>, Option<(usize, usize)>) {
    parse_at_depth(source, 0)
}

fn parse_at_depth(source: &str, depth: usize) -> (Vec<Block>, Option<(usize, usize)>) {
    if crate::widgets::source_too_large(source) {
        return (vec![crate::widgets::oversize_block()], None);
    }
    let mut lines = Vec::new();
    let mut ends = Vec::new();
    let mut offset = 0;
    for raw in source.split_inclusive('\n') {
        offset += raw.len();
        lines.push(raw.trim_end());
        ends.push(raw.ends_with('\n').then_some(offset));
    }
    let mut blocks = Blocks {
        depth,
        ..Blocks::default()
    };
    let mut boundary = None;
    let mut i = 0;
    while i < lines.len() {
        let eaten = blocks.step(&lines, i).max(1);
        if eaten == 1
            && blocks.fence.is_none()
            && lines.get(i).is_some_and(|line| line.trim().is_empty())
            && let Some(Some(end)) = ends.get(i)
        {
            boundary = Some((*end, blocks.out.len()));
        }
        i += eaten;
    }
    (blocks.finish(), boundary)
}

pub(super) fn parse_nested(source: &str, depth: usize) -> Vec<Block> {
    if depth < MAX_NESTING {
        return parse_at_depth(source, depth).0;
    }
    let text = source.split_whitespace().collect::<Vec<_>>().join(" ");
    vec![Block::Paragraph(text)]
}

fn fence_open(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("```")?;
    Some(rest.trim_start_matches('`').trim().to_string())
}

impl Blocks {
    fn step(&mut self, lines: &[&str], i: usize) -> usize {
        let Some(&line) = lines.get(i) else {
            return 1;
        };
        if self.feed_fence(line) {
            return 1;
        }
        if let Some(lang) = fence_open(line) {
            self.flush();
            self.fence = Some((lang, String::new()));
            return 1;
        }
        if rule(line) {
            self.flush();
            self.out.push(Block::Rule);
            return 1;
        }
        if let Some(eaten) = self.container(lines, i) {
            return eaten;
        }
        if let Some((level, heading)) = heading(line) {
            self.flush();
            self.out.push(Block::Heading {
                level,
                text: heading.to_string(),
            });
            return 1;
        }
        if line.trim().is_empty() {
            self.flush();
            return 1;
        }
        if !self.para.is_empty() {
            self.para.push(' ');
        }
        self.para.push_str(line.trim());
        1
    }

    fn container(&mut self, lines: &[&str], i: usize) -> Option<usize> {
        let line = lines.get(i).copied()?;
        let (block, eaten) = if let Some((quote, eaten)) = take_quote(lines, i) {
            (Block::Quote(parse_nested(&quote, self.depth + 1)), eaten)
        } else if let Some((rows, eaten)) = take_table(lines, i) {
            (Block::Table(rows), eaten)
        } else if let Some((alt, src)) = image_line(line) {
            let (alt, src) = (alt.to_string(), src.to_string());
            (Block::Image { alt, src }, 1)
        } else if list_marker(line).is_some() {
            let (list, eaten) = take_list(lines, i, self.depth + 1)?;
            (Block::List(list), eaten)
        } else {
            return None;
        };
        self.flush();
        self.out.push(block);
        Some(eaten)
    }

    fn feed_fence(&mut self, line: &str) -> bool {
        let Some((lang, buf)) = self.fence.as_mut() else {
            return false;
        };
        if line.trim_start().starts_with("```") {
            let body = std::mem::take(buf);
            let lang = std::mem::take(lang);
            self.out.push(fence_block(&lang, body));
            self.fence = None;
        } else {
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf.push_str(line);
        }
        true
    }

    fn flush(&mut self) {
        let trimmed = self.para.trim();
        if !trimmed.is_empty() {
            self.out.push(Block::Paragraph(trimmed.to_string()));
        }
        self.para.clear();
    }

    fn finish(mut self) -> Vec<Block> {
        if let Some((lang, buf)) = self.fence.take() {
            self.out.push(fence_block(&lang, buf));
        }
        self.flush();
        self.out
    }
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
