use std::ops::Range;

use super::events::Located;
use super::parse::{Block, Inline, nested};
use crate::widgets::text_search::find_ci;

enum Piece {
    Open(String),
    Close,
    Text(String),
}

struct Open {
    start: usize,
    summary: Vec<Inline>,
    blocks: Vec<Block>,
}

#[derive(Default)]
struct Fold {
    out: Vec<Located>,
    open: Vec<Open>,
}

pub(super) fn fold_blocks(blocks: Vec<Block>) -> Vec<Block> {
    if !blocks.iter().any(|block| matches!(block, Block::Html(_))) {
        return blocks;
    }
    let located = blocks.into_iter().map(|block| (0..0, block)).collect();
    fold(located).into_iter().map(|(_, block)| block).collect()
}

pub(super) fn fold(items: Vec<Located>) -> Vec<Located> {
    let mut fold = Fold::default();
    let mut end = 0;
    for (range, block) in items {
        end = range.end;
        match block {
            Block::Html(html) if find_ci(&html, "<details").is_some() || find_ci(&html, "</details").is_some() => {
                for piece in pieces(&html) {
                    fold.piece(piece, &range);
                }
            }
            other => fold.push(other, range),
        }
    }
    while !fold.open.is_empty() {
        fold.close(end);
    }
    fold.out
}

impl Fold {
    fn push(&mut self, block: Block, range: Range<usize>) {
        match self.open.last_mut() {
            Some(open) => open.blocks.push(block),
            None => self.out.push((range, block)),
        }
    }

    fn piece(&mut self, piece: Piece, range: &Range<usize>) {
        match piece {
            Piece::Open(summary) => self.open.push(Open {
                start: range.start,
                summary: summary_inlines(&summary),
                blocks: Vec::new(),
            }),
            Piece::Close => self.close(range.end),
            Piece::Text(text) => {
                for block in nested(&text) {
                    self.push(block, range.clone());
                }
            }
        }
    }

    fn close(&mut self, end: usize) {
        let Some(open) = self.open.pop() else {
            return;
        };
        let block = Block::Details {
            summary: open.summary,
            blocks: open.blocks,
        };
        self.push(block, open.start..end);
    }
}

fn summary_inlines(summary: &str) -> Vec<Inline> {
    let text = summary.trim();
    let text = if text.is_empty() { "Details" } else { text };
    vec![Inline::Text(text.to_string())]
}

fn pieces(html: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut rest = html;
    loop {
        let open = find_ci(rest, "<details");
        let close = find_ci(rest, "</details>");
        let at = match (open, close) {
            (Some(open), Some(close)) => open.min(close),
            (Some(at), None) | (None, Some(at)) => at,
            (None, None) => break,
        };
        text_piece(&mut out, &rest[..at]);
        rest = &rest[at..];
        if close == Some(at) {
            out.push(Piece::Close);
            rest = &rest["</details>".len()..];
            continue;
        }
        let tag_end = rest.find('>').map_or(rest.len(), |end| end + 1);
        rest = &rest[tag_end..];
        let (summary, after) = take_summary(rest);
        out.push(Piece::Open(summary));
        rest = after;
    }
    text_piece(&mut out, rest);
    out
}

fn take_summary(rest: &str) -> (String, &str) {
    let trimmed = rest.trim_start();
    if find_ci(trimmed, "<summary") != Some(0) {
        return (String::new(), rest);
    }
    let body_start = trimmed.find('>').map_or(trimmed.len(), |end| end + 1);
    let body = &trimmed[body_start..];
    match find_ci(body, "</summary>") {
        Some(end) => (strip_tags(&body[..end]), &body[end + "</summary>".len()..]),
        None => (strip_tags(body), ""),
    }
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut inside = false;
    for ch in html.chars() {
        match ch {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(ch),
            _ => {}
        }
    }
    out
}

fn text_piece(out: &mut Vec<Piece>, text: &str) {
    if !text.trim().is_empty() {
        out.push(Piece::Text(text.to_string()));
    }
}
