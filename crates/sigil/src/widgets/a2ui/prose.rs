use std::fmt::Write as _;

use crate::kernel::TextBuilder;
use crate::widgets::markdown::parse::{Inline, parse_inlines};
use crate::widgets::markdown_inline;

fn linked(spans: &[Inline]) -> bool {
    spans.iter().any(|span| match span {
        Inline::Link { .. } | Inline::Image { .. } => true,
        Inline::Strong(body) | Inline::Em(body) | Inline::Strike(body) => linked(body),
        Inline::Text(_) | Inline::Code(_) | Inline::Math(_) | Inline::Note(_) => false,
    })
}

fn escape(out: &mut String, text: &str) {
    for character in text.chars() {
        if character.is_ascii_punctuation() {
            out.push('\\');
        }
        out.push(character);
    }
}

fn wrap(out: &mut String, marker: &str, body: &[Inline]) {
    out.push_str(marker);
    emit(out, body);
    out.push_str(marker);
}

pub(super) fn longest_tick_run(text: &str) -> usize {
    text.bytes()
        .fold((0, 0), |(longest, run), byte| {
            let run = if byte == b'`' { run + 1 } else { 0 };
            (longest.max(run), run)
        })
        .0
}

fn code(out: &mut String, body: &str) {
    let fence = "`".repeat(longest_tick_run(body) + 1);
    let _ = write!(out, "{fence} {body} {fence}");
}

fn emit(out: &mut String, spans: &[Inline]) {
    for span in spans {
        match span {
            Inline::Text(text) | Inline::Math(text) => escape(out, text),
            Inline::Strong(body) => wrap(out, "**", body),
            Inline::Em(body) => wrap(out, "*", body),
            Inline::Strike(body) => wrap(out, "~~", body),
            Inline::Code(body) => code(out, body),
            Inline::Note(label) => escape(out, &format!("[^{label}]")),
            Inline::Image { alt, .. } => escape(out, &format!("[{alt}]")),
            Inline::Link { body, .. } => emit(out, body),
        }
    }
}

pub(super) fn plain_links(source: &str) -> TextBuilder {
    let spans = parse_inlines(source);
    if !linked(&spans) {
        return markdown_inline(source);
    }
    let mut out = String::new();
    emit(&mut out, &spans);
    markdown_inline(&out)
}
