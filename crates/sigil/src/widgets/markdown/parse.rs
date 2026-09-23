#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Block {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    List(Vec<String>),
    Code(String),
    Mermaid(String),
    Chart {
        title: Option<String>,
        rows: Vec<(String, f64)>,
    },
    Image {
        alt: String,
        src: String,
    },
    Table(Vec<Vec<String>>),
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

#[derive(Default)]
struct Blocks {
    out: Vec<Block>,
    para: String,
    list: Vec<String>,
    fence: Option<(String, String)>,
}

pub(crate) fn parse_blocks(source: &str) -> Vec<Block> {
    if crate::widgets::source_too_large(source) {
        return vec![Block::Paragraph(String::from(
            "Source is larger than the 1 MiB limit.",
        ))];
    }
    let lines: Vec<&str> = source.lines().map(str::trim_end).collect();
    let mut blocks = Blocks::default();
    let mut i = 0;
    while i < lines.len() {
        i += blocks.step(&lines, i);
    }
    blocks.finish()
}

impl Blocks {
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
        if let Some((alt, src)) = image_line(line) {
            self.flush_all();
            self.out.push(Block::Image {
                alt: alt.to_string(),
                src: src.to_string(),
            });
            return 1;
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
    } else if matches!(lang, "chart" | "xychart" | "xychart-beta") {
        chart_block(&body, lang.starts_with("xychart"))
    } else {
        Block::Code(body)
    }
}

type ChartData = (Option<String>, Vec<(String, f64)>);

fn chart_block(body: &str, xychart: bool) -> Block {
    chart_rows(body, xychart).map_or_else(
        || Block::Mermaid(body.to_string()),
        |(title, rows)| Block::Chart { title, rows },
    )
}

fn chart_rows(source: &str, xychart: bool) -> Option<ChartData> {
    let lines: Vec<&str> = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("%%"))
        .collect();
    if xychart
        || lines
            .first()
            .is_some_and(|line| line.starts_with("xychart"))
    {
        return xychart_rows(if xychart { &lines } else { lines.get(1..)? });
    }
    let mut title = None;
    let mut rows = Vec::new();
    for line in lines {
        if rows.is_empty()
            && line
                .split_whitespace()
                .all(|word| word.parse::<f64>().is_err())
            && !line.contains(':')
            && !line.contains(',')
        {
            title = Some(line.trim_matches('"').to_string());
            continue;
        }
        rows.push(pair(line)?);
    }
    (!rows.is_empty()).then_some((title, rows))
}

fn xychart_rows(lines: &[&str]) -> Option<ChartData> {
    let mut title = None;
    let mut labels = Vec::new();
    let mut values = Vec::new();
    for line in lines {
        if let Some(rest) = line.strip_prefix("title") {
            title = Some(rest.trim().trim_matches('"').to_string());
        } else if line.starts_with("x-axis") {
            labels = csv_items(line);
        } else if line.starts_with("bar") || line.starts_with("line") {
            values = csv_items(line)
                .into_iter()
                .filter_map(|item| item.parse::<f64>().ok())
                .collect();
        }
    }
    if values.is_empty() {
        return None;
    }
    let rows = values
        .iter()
        .enumerate()
        .map(|(i, value)| {
            (
                labels
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("{}", i + 1)),
                *value,
            )
        })
        .collect();
    Some((title, rows))
}

fn csv_items(line: &str) -> Vec<String> {
    let Some(open) = line.find('[') else {
        return Vec::new();
    };
    let Some(close) = line.rfind(']') else {
        return Vec::new();
    };
    if close <= open {
        return Vec::new();
    }
    line[open + 1..close]
        .split(',')
        .map(|item| item.trim().trim_matches('"').to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn pair(line: &str) -> Option<(String, f64)> {
    if let Some((label, value)) = line.rsplit_once(':').or_else(|| line.rsplit_once(',')) {
        let value: f64 = value.trim().parse().ok()?;
        let label = label.trim().trim_matches('"').to_string();
        return (!label.is_empty() && value >= 0.0).then_some((label, value));
    }
    let value = line.split_whitespace().next_back()?;
    let label = line[..line.len() - value.len()].trim().trim_matches('"');
    let value: f64 = value.parse().ok()?;
    (!label.is_empty() && value >= 0.0).then_some((label.to_string(), value))
}

fn image_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("![")?;
    let close = rest.find("](")?;
    let src = rest[close + 2..].strip_suffix(')')?;
    Some((&rest[..close], src))
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
#[path = "parse_tests.rs"]
mod tests;
