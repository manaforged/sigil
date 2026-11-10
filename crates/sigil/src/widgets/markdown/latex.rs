use std::borrow::Cow;
use std::ops::Range;

use pulldown_cmark::{Event, Parser, Tag};

use super::events::OPTIONS;

const MAX_MATH_BYTES: usize = 512;

struct Pair {
    open: &'static str,
    close: &'static str,
    open_as: &'static str,
    close_as: &'static str,
}

const INLINE: Pair = Pair {
    open: "\\(",
    close: "\\)",
    open_as: "${",
    close_as: "}$",
};

const DISPLAY: Pair = Pair {
    open: "\\[",
    close: "\\]",
    open_as: "$$",
    close_as: "$$",
};

pub(super) fn delimiters(source: &str) -> Cow<'_, str> {
    if !source.contains(INLINE.open) && !source.contains(DISPLAY.open) {
        return Cow::Borrowed(source);
    }
    let code = code_ranges(source);
    let mut out = source.to_string();
    let mut at = 0;
    while let Some(open) = next_open(source, at, &code) {
        let pair = if source[open..].starts_with(INLINE.open) {
            &INLINE
        } else {
            &DISPLAY
        };
        let body = open + 2;
        let close = source[body..]
            .find(pair.close)
            .map(|end| end + body)
            .filter(|&end| end - body <= MAX_MATH_BYTES && !source[body..end].contains("\n\n"))
            .filter(|&end| !inside(&code, end));
        let Some(close) = close else {
            at = body;
            continue;
        };
        out.replace_range(open..body, pair.open_as);
        out.replace_range(close..close + 2, pair.close_as);
        at = close + 2;
    }
    Cow::Owned(out)
}

fn next_open(source: &str, from: usize, code: &[Range<usize>]) -> Option<usize> {
    let mut at = from;
    while let Some(found) = source.get(at..)?.find('\\') {
        let index = at + found;
        let rest = &source[index..];
        let escaped = source[..index].ends_with('\\');
        if !escaped
            && (rest.starts_with(INLINE.open) || rest.starts_with(DISPLAY.open))
            && !inside(code, index)
        {
            return Some(index);
        }
        at = index + 1;
    }
    None
}

fn inside(code: &[Range<usize>], at: usize) -> bool {
    code.iter().any(|range| range.contains(&at))
}

fn code_ranges(source: &str) -> Vec<Range<usize>> {
    Parser::new_ext(source, OPTIONS)
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            Event::Code(_)
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::Start(Tag::CodeBlock(_)) => Some(range),
            _ => None,
        })
        .collect()
}
