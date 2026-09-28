use std::fmt::Write as _;

use crate::widgets::markdown::parse::{Block, List};
use crate::widgets::text_search::{CloseTags, find_ci, starts_ci};

pub(crate) fn parse_html(source: &str) -> Vec<Block> {
    if crate::widgets::source_too_large(source) {
        return vec![crate::widgets::oversize_block()];
    }
    let body = strip_noise(source);
    let mut out = Vec::new();
    let mut rest = body.as_str();
    let mut tags = CloseTags::default();
    while !rest.is_empty() {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        if let Some((block, eaten)) = next_block(rest, &mut tags) {
            out.push(block);
            rest = &rest[eaten..];
            continue;
        }
        let (text, eaten) = take_text(rest);
        if !text.trim().is_empty() {
            out.push(Block::Paragraph(text));
        }
        rest = &rest[eaten..];
    }
    out
}

fn strip_noise(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let Some(end) = after.find('>') else {
            rest = after;
            break;
        };
        let script = starts_ci(after, "<script");
        if script || starts_ci(after, "<style") {
            let close = if script { "</script>" } else { "</style>" };
            rest = after.get(end + 1..).unwrap_or("");
            if let Some(idx) = find_ci(rest, close) {
                rest = rest.get(idx + close.len()..).unwrap_or("");
            } else {
                rest = "";
            }
            continue;
        }
        out.push_str(&after[..=end]);
        rest = after.get(end + 1..).unwrap_or("");
    }
    out.push_str(rest);
    out
}

fn next_block(rest: &str, tags: &mut CloseTags) -> Option<(Block, usize)> {
    heading(rest, tags)
        .or_else(|| list(rest, "ul", tags))
        .or_else(|| list(rest, "ol", tags))
        .or_else(|| pre(rest, tags))
        .or_else(|| table(rest, tags))
        .or_else(|| paragraph(rest, tags))
}

fn heading(rest: &str, tags: &mut CloseTags) -> Option<(Block, usize)> {
    for level in 1..=3_u8 {
        let open = format!("<h{level}>");
        let close = format!("</h{level}>");
        if let Some((inner, eaten)) = tagged(rest, (&open, &close), tags) {
            return Some((
                Block::Heading {
                    level,
                    text: inner_text(&inner),
                },
                eaten,
            ));
        }
    }
    None
}

fn paragraph(rest: &str, tags: &mut CloseTags) -> Option<(Block, usize)> {
    let (inner, eaten) = tagged(rest, ("<p>", "</p>"), tags)?;
    Some((Block::Paragraph(inner_text(&inner)), eaten))
}

fn pre(rest: &str, tags: &mut CloseTags) -> Option<(Block, usize)> {
    let (inner, eaten) = tagged(rest, ("<pre>", "</pre>"), tags)?;
    let text = tagged(&inner, ("<code>", "</code>"), &mut CloseTags::default())
        .map_or_else(|| decode(&inner), |(code, _)| decode(&code));
    Some((
        Block::Code {
            lang: String::new(),
            body: text,
        },
        eaten,
    ))
}

fn list(rest: &str, tag: &str, tags: &mut CloseTags) -> Option<(Block, usize)> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let (inner, eaten) = tagged(rest, (&open, &close), tags)?;
    let mut items = Vec::new();
    let mut cursor = inner.as_str();
    let mut inner_tags = CloseTags::default();
    while let Some((item, used)) = tagged(cursor.trim_start(), ("<li>", "</li>"), &mut inner_tags) {
        items.push(inner_text(&item));
        let skip = cursor.len() - cursor.trim_start().len();
        cursor = cursor.get(skip + used..).unwrap_or("");
    }
    let items = items
        .into_iter()
        .map(|item| vec![Block::Paragraph(item)])
        .collect();
    let start = (tag == "ol").then_some(1);
    Some((Block::List(List { start, items }), eaten))
}

fn table(rest: &str, tags: &mut CloseTags) -> Option<(Block, usize)> {
    let (inner, eaten) = tagged(rest, ("<table>", "</table>"), tags)?;
    let mut rows = Vec::new();
    let mut cursor = inner.as_str();
    let mut inner_tags = CloseTags::default();
    while let Some((row, used)) = tagged(cursor.trim_start(), ("<tr>", "</tr>"), &mut inner_tags) {
        rows.push(row_cells(&row));
        let skip = cursor.len() - cursor.trim_start().len();
        cursor = cursor.get(skip + used..).unwrap_or("");
    }
    Some((Block::Table(rows), eaten))
}

fn row_cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cursor = row;
    let mut tags = CloseTags::default();
    loop {
        cursor = cursor.trim_start();
        let next = tagged(cursor, ("<th>", "</th>"), &mut tags)
            .or_else(|| tagged(cursor, ("<td>", "</td>"), &mut tags));
        let Some((cell, used)) = next else {
            break;
        };
        cells.push(inner_text(&cell));
        cursor = cursor.get(used..).unwrap_or("");
    }
    cells
}

fn tagged(rest: &str, pair: (&str, &str), tags: &mut CloseTags) -> Option<(String, usize)> {
    let (open, close) = pair;
    if !starts_ci(rest, open) {
        return None;
    }
    let inner_at = open.len();
    let rel = tags.find(rest.get(inner_at..)?, close)?;
    let inner = rest.get(inner_at..inner_at + rel)?.to_string();
    Some((inner, inner_at + rel + close.len()))
}

fn take_text(rest: &str) -> (String, usize) {
    match rest.find('<').filter(|&i| rest[i..].contains('>')) {
        Some(0) => {
            let end = rest.find('>').map_or(rest.len(), |i| i + 1);
            (String::new(), end)
        }
        Some(i) => (inner_text(&rest[..i]), i),
        None => (inner_text(rest), rest.len()),
    }
}

fn inner_text(source: &str) -> String {
    let mut out = String::new();
    let mut rest = source;
    let mut tags = CloseTags::default();
    while !rest.is_empty() {
        if rest.starts_with('<') {
            if let Some((inner, eaten, kind)) = inline_tag(rest, &mut tags) {
                let body = inner_text(&inner);
                match kind {
                    Inline::Strong => drop(write!(out, "**{body}**")),
                    Inline::Em => drop(write!(out, "*{body}*")),
                    Inline::Code => drop(write!(out, "`{body}`")),
                    Inline::Link(href) => drop(write!(out, "[{body}]({href})")),
                }
                rest = rest.get(eaten..).unwrap_or("");
                continue;
            }
            let Some(close) = rest.find('>') else {
                out.push_str(&decode(rest));
                break;
            };
            rest = rest.get(close + 1..).unwrap_or("");
            continue;
        }
        let end = rest.find('<').unwrap_or(rest.len());
        out.push_str(&decode(&rest[..end]));
        rest = rest.get(end..).unwrap_or("");
    }
    out
}

enum Inline {
    Strong,
    Em,
    Code,
    Link(String),
}

fn inline_tag(rest: &str, tags: &mut CloseTags) -> Option<(String, usize, Inline)> {
    let pairs = [
        ("<strong>", "</strong>", Inline::Strong),
        ("<b>", "</b>", Inline::Strong),
        ("<em>", "</em>", Inline::Em),
        ("<i>", "</i>", Inline::Em),
        ("<code>", "</code>", Inline::Code),
    ];
    for (open, close, kind) in pairs {
        if let Some((inner, eaten)) = tagged(rest, (open, close), tags) {
            return Some((inner, eaten, kind));
        }
    }
    if starts_ci(rest, "<a ") || starts_ci(rest, "<a>") {
        let gt = rest.find('>')?;
        let open = &rest[..=gt];
        let href = attr(open, "href").unwrap_or_default();
        let close = "</a>";
        let rel = tags.find(rest.get(gt + 1..)?, close)?;
        let inner = rest.get(gt + 1..gt + 1 + rel)?.to_string();
        Some((inner, gt + 1 + rel + close.len(), Inline::Link(href)))
    } else {
        None
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=");
    let at = find_ci(tag, &needle)?;
    let after = tag.get(at + needle.len()..)?;
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = after.get(1..)?;
    let end = rest.find(quote)?;
    Some(decode(&rest[..end]))
}

fn decode(source: &str) -> String {
    source
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
}
