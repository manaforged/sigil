//! Linear markdown parse: blocks then inlines.

/// One block after a linear CommonMark-shaped pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Block {
    Heading { level: u8, text: String },
    Paragraph(String),
    List(Vec<String>),
    Code(String),
    Mermaid(String),
    Table(Vec<Vec<String>>),
}

/// Inline run inside a heading, paragraph, or list item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Span {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    Link { text: String, href: String },
}

/// Block-split state: finished blocks plus the open paragraph/list/fence.
#[derive(Default)]
struct Blocks {
    out: Vec<Block>,
    para: String,
    list: Vec<String>,
    fence: Option<(String, String)>,
}

/// Split `source` into heading / paragraph / list / fence / table blocks.
pub(crate) fn parse_blocks(source: &str) -> Vec<Block> {
    let lines: Vec<&str> = source.lines().map(str::trim_end).collect();
    let mut blocks = Blocks::default();
    let mut i = 0;
    while i < lines.len() {
        i += blocks.step(&lines, i);
    }
    blocks.finish()
}

impl Blocks {
    /// Consume the line at `i`; returns how many lines were eaten.
    fn step(&mut self, lines: &[&str], i: usize) -> usize {
        let Some(&line) = lines.get(i) else {
            return 1;
        };
        if self.feed_fence(line) {
            return 1;
        }
        if line.starts_with("```") {
            self.flush_all();
            let lang = line.trim_start_matches('`').trim().to_string();
            self.fence = Some((lang, String::new()));
            return 1;
        }
        if let Some((rows, eaten)) = take_table(lines, i) {
            self.flush_all();
            self.out.push(Block::Table(rows));
            return eaten;
        }
        if let Some(item) = list_item(line) {
            self.flush_para();
            self.list.push(item.to_string());
            return 1;
        }
        if let Some((level, heading)) = heading(line) {
            self.flush_all();
            self.out.push(Block::Heading {
                level,
                text: heading.to_string(),
            });
            return 1;
        }
        if line.trim().is_empty() {
            self.flush_all();
            return 1;
        }
        self.flush_list();
        if !self.para.is_empty() {
            self.para.push('\n');
        }
        self.para.push_str(line.trim());
        1
    }

    /// Feed `line` into the open fence. Returns false when no fence is open.
    fn feed_fence(&mut self, line: &str) -> bool {
        let Some((lang, buf)) = self.fence.as_mut() else {
            return false;
        };
        if line.starts_with("```") {
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

    fn flush_para(&mut self) {
        let trimmed = self.para.trim();
        if !trimmed.is_empty() {
            self.out.push(Block::Paragraph(trimmed.to_string()));
        }
        self.para.clear();
    }

    fn flush_list(&mut self) {
        if !self.list.is_empty() {
            self.out.push(Block::List(std::mem::take(&mut self.list)));
        }
    }

    fn flush_all(&mut self) {
        self.flush_para();
        self.flush_list();
    }

    fn finish(mut self) -> Vec<Block> {
        if let Some((lang, buf)) = self.fence.take() {
            self.out.push(fence_block(&lang, buf));
        }
        self.flush_para();
        self.flush_list();
        self.out
    }
}

fn fence_block(lang: &str, body: String) -> Block {
    if lang == "mermaid" {
        Block::Mermaid(body)
    } else {
        Block::Code(body)
    }
}

fn take_table(lines: &[&str], start: usize) -> Option<(Vec<Vec<String>>, usize)> {
    let header = pipe_row(lines.get(start).copied()?)?;
    let sep = lines.get(start + 1).copied()?;
    if !is_table_sep(sep) {
        return None;
    }
    let mut rows = vec![header];
    let mut i = start + 2;
    while i < lines.len() {
        let Some(row) = lines.get(i).copied().and_then(pipe_row) else {
            break;
        };
        rows.push(row);
        i += 1;
    }
    Some((rows, i - start))
}

fn pipe_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }
    let cells: Vec<String> = trimmed
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect();
    if cells.is_empty() {
        return None;
    }
    Some(cells)
}

fn is_table_sep(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return false;
    }
    trimmed.trim_matches('|').split('|').all(|cell| {
        let cell = cell.trim();
        !cell.is_empty() && cell.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
    })
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let rest = line.strip_prefix('#')?;
    let mut level = 1_u8;
    let mut body = rest;
    while let Some(next) = body.strip_prefix('#') {
        level = level.saturating_add(1);
        body = next;
        if level == 3 {
            break;
        }
    }
    let body = body.strip_prefix(' ')?;
    Some((level, body.trim()))
}

fn list_item(line: &str) -> Option<&str> {
    let body = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))?;
    Some(body.trim())
}

/// Split a line into text / bold / italic / code / link runs.
pub(crate) fn parse_inlines(source: &str) -> Vec<Span> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut text = String::new();
    while i < bytes.len() {
        if bytes.get(i) == Some(&b'`')
            && let Some(end) = source[i + 1..].find('`')
        {
            flush_text(&mut out, &mut text);
            out.push(Span::Code(source[i + 1..i + 1 + end].to_string()));
            i += end + 2;
            continue;
        }
        if bytes.get(i) == Some(&b'[')
            && let Some((label, href, eat)) = link_at(&source[i..])
        {
            flush_text(&mut out, &mut text);
            out.push(Span::Link { text: label, href });
            i += eat;
            continue;
        }
        if source[i..].starts_with("**")
            && let Some(end) = source[i + 2..].find("**")
        {
            flush_text(&mut out, &mut text);
            out.push(Span::Bold(source[i + 2..i + 2 + end].to_string()));
            i += end + 4;
            continue;
        }
        if bytes.get(i) == Some(&b'*')
            && let Some(end) = source[i + 1..].find('*')
        {
            flush_text(&mut out, &mut text);
            out.push(Span::Italic(source[i + 1..i + 1 + end].to_string()));
            i += end + 2;
            continue;
        }
        let ch = source[i..].chars().next().unwrap_or(' ');
        text.push(ch);
        i += ch.len_utf8();
    }
    flush_text(&mut out, &mut text);
    if out.is_empty() {
        out.push(Span::Text(String::new()));
    }
    out
}

fn flush_text(out: &mut Vec<Span>, text: &mut String) {
    if !text.is_empty() {
        out.push(Span::Text(std::mem::take(text)));
    }
}

fn link_at(source: &str) -> Option<(String, String, usize)> {
    let rest = source.strip_prefix('[')?;
    let close = rest.find(']')?;
    let label = rest[..close].to_string();
    let after = rest.get(close + 1..)?;
    let href = after.strip_prefix('(')?;
    let end = href.find(')')?;
    let url = href[..end].to_string();
    Some((label, url, 1 + close + 1 + 1 + end + 1))
}

#[cfg(test)]
mod tests {
    use super::{Block, Span, parse_blocks, parse_inlines};

    #[test]
    fn parse_headings_list_fence_and_paragraph() {
        let blocks =
            parse_blocks("# Title\n\nHello **world**.\n\n- one\n- two\n\n```\nfn x() {}\n```\n");
        assert_eq!(
            blocks,
            vec![
                Block::Heading {
                    level: 1,
                    text: "Title".into(),
                },
                Block::Paragraph("Hello **world**.".into()),
                Block::List(vec!["one".into(), "two".into()]),
                Block::Code("fn x() {}".into()),
            ]
        );
        assert_eq!(
            parse_inlines("Hello **world**."),
            vec![
                Span::Text("Hello ".into()),
                Span::Bold("world".into()),
                Span::Text(".".into()),
            ]
        );
        assert_eq!(
            parse_inlines("see [docs](https://example.com)"),
            vec![
                Span::Text("see ".into()),
                Span::Link {
                    text: "docs".into(),
                    href: "https://example.com".into(),
                },
            ]
        );
        assert_eq!(
            parse_blocks("| a | b |\n| --- | --- |\n| 1 | 2 |\n"),
            vec![Block::Table(vec![
                vec!["a".into(), "b".into()],
                vec!["1".into(), "2".into()],
            ])]
        );
    }
}
