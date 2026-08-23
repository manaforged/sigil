//! HTML subset → the same [`Block`] values markdown produces.

use std::fmt::Write as _;

use crate::markdown::parse::Block;

/// Map a document subset to markdown blocks. Scripts and styles drop.
pub(crate) fn parse_html(source: &str) -> Vec<Block> {
    let body = strip_noise(source);
    let mut out = Vec::new();
    let mut rest = body.as_str();
    while !rest.is_empty() {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        if let Some((block, eaten)) = next_block(rest) {
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
            break;
        };
        let tag = after[..=end].to_ascii_lowercase();
        if tag.starts_with("<script") || tag.starts_with("<style") {
            let close = if tag.starts_with("<script") {
                "</script>"
            } else {
                "</style>"
            };
            rest = after.get(end + 1..).unwrap_or("");
            if let Some(idx) = rest.to_ascii_lowercase().find(close) {
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

fn next_block(rest: &str) -> Option<(Block, usize)> {
    heading(rest)
        .or_else(|| list(rest, "ul"))
        .or_else(|| list(rest, "ol"))
        .or_else(|| pre(rest))
        .or_else(|| table(rest))
        .or_else(|| paragraph(rest))
}

fn heading(rest: &str) -> Option<(Block, usize)> {
    for level in 1..=3_u8 {
        let open = format!("<h{level}>");
        let close = format!("</h{level}>");
        if let Some((inner, eaten)) = tagged(rest, &open, &close) {
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

fn paragraph(rest: &str) -> Option<(Block, usize)> {
    let (inner, eaten) = tagged(rest, "<p>", "</p>")?;
    Some((Block::Paragraph(inner_text(&inner)), eaten))
}

fn pre(rest: &str) -> Option<(Block, usize)> {
    let (inner, eaten) = tagged(rest, "<pre>", "</pre>")?;
    let text = tagged(&inner, "<code>", "</code>")
        .map_or_else(|| decode(&inner), |(code, _)| decode(&code));
    Some((Block::Code(text), eaten))
}

fn list(rest: &str, tag: &str) -> Option<(Block, usize)> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let (inner, eaten) = tagged(rest, &open, &close)?;
    let mut items = Vec::new();
    let mut cursor = inner.as_str();
    while let Some((item, used)) = tagged(cursor.trim_start(), "<li>", "</li>") {
        items.push(inner_text(&item));
        let skip = cursor.len() - cursor.trim_start().len();
        cursor = cursor.get(skip + used..).unwrap_or("");
    }
    Some((Block::List(items), eaten))
}

fn table(rest: &str) -> Option<(Block, usize)> {
    let (inner, eaten) = tagged(rest, "<table>", "</table>")?;
    let mut rows = Vec::new();
    let mut cursor = inner.as_str();
    while let Some((row, used)) = tagged(cursor.trim_start(), "<tr>", "</tr>") {
        rows.push(row_cells(&row));
        let skip = cursor.len() - cursor.trim_start().len();
        cursor = cursor.get(skip + used..).unwrap_or("");
    }
    Some((Block::Table(rows), eaten))
}

fn row_cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cursor = row;
    loop {
        cursor = cursor.trim_start();
        let next = tagged(cursor, "<th>", "</th>").or_else(|| tagged(cursor, "<td>", "</td>"));
        let Some((cell, used)) = next else {
            break;
        };
        cells.push(inner_text(&cell));
        cursor = cursor.get(used..).unwrap_or("");
    }
    cells
}

fn tagged(rest: &str, open: &str, close: &str) -> Option<(String, usize)> {
    let lower = rest.to_ascii_lowercase();
    if !lower.starts_with(open) {
        return None;
    }
    let inner_at = open.len();
    let rel = lower.get(inner_at..)?.find(close)?;
    let inner = rest.get(inner_at..inner_at + rel)?.to_string();
    Some((inner, inner_at + rel + close.len()))
}

fn take_text(rest: &str) -> (String, usize) {
    match rest.find('<') {
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
    while !rest.is_empty() {
        if rest.starts_with('<') {
            if let Some((inner, eaten, kind)) = inline_tag(rest) {
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
            let skip = rest.find('>').map_or(rest.len(), |i| i + 1);
            rest = rest.get(skip..).unwrap_or("");
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

fn inline_tag(rest: &str) -> Option<(String, usize, Inline)> {
    let pairs = [
        ("<strong>", "</strong>", Inline::Strong),
        ("<b>", "</b>", Inline::Strong),
        ("<em>", "</em>", Inline::Em),
        ("<i>", "</i>", Inline::Em),
        ("<code>", "</code>", Inline::Code),
    ];
    for (open, close, kind) in pairs {
        if let Some((inner, eaten)) = tagged(rest, open, close) {
            return Some((inner, eaten, kind));
        }
    }
    let lower = rest.to_ascii_lowercase();
    if lower.starts_with("<a ") || lower.starts_with("<a>") {
        let gt = rest.find('>')?;
        let open = &rest[..=gt];
        let href = attr(open, "href").unwrap_or_default();
        let close = "</a>";
        let rel = lower.get(gt + 1..)?.find(close)?;
        let inner = rest.get(gt + 1..gt + 1 + rel)?.to_string();
        Some((inner, gt + 1 + rel + close.len(), Inline::Link(href)))
    } else {
        None
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=");
    let lower = tag.to_ascii_lowercase();
    let at = lower.find(&needle)?;
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
