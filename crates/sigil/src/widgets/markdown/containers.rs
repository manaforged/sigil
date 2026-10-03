use super::parse::{Block, parse_nested};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct List {
    pub(crate) start: Option<u64>,
    pub(crate) items: Vec<ListItem>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Task {
    Open,
    Done,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ListItem {
    pub(crate) task: Option<Task>,
    pub(crate) blocks: Vec<Block>,
}

impl ListItem {
    pub(crate) const fn plain(blocks: Vec<Block>) -> Self {
        Self { task: None, blocks }
    }
}

fn task_marker(line: &str) -> Option<(Task, &str)> {
    let task = match line.get(..4)? {
        "[ ] " => Task::Open,
        "[x] " | "[X] " => Task::Done,
        _ => return None,
    };
    Some((task, &line[4..]))
}

fn list_item(lines: &[String], depth: usize) -> ListItem {
    let source = lines.join("\n");
    match task_marker(&source) {
        Some((task, rest)) => ListItem {
            task: Some(task),
            blocks: parse_nested(rest, depth),
        },
        None => ListItem {
            task: None,
            blocks: parse_nested(&source, depth),
        },
    }
}

#[derive(Clone, Copy)]
pub(super) struct Marker {
    indent: usize,
    number: Option<u64>,
    content: usize,
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn dedent(line: &str, columns: usize) -> &str {
    let strip = indent(line).min(columns);
    line.get(strip..).unwrap_or_default()
}

pub(super) fn list_marker(line: &str) -> Option<Marker> {
    let lead = indent(line);
    let rest = line.get(lead..)?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let (number, width) = if digits == 0 {
        rest.starts_with(['-', '*', '+']).then_some((None, 1))?
    } else if digits <= 9 && rest[digits..].starts_with(['.', ')']) {
        (rest[..digits].parse().ok(), digits + 1)
    } else {
        return None;
    };
    let after = rest.get(width..)?;
    if !after.is_empty() && !after.starts_with(' ') {
        return None;
    }
    let gap = (after.len() - after.trim_start_matches(' ').len()).clamp(1, 4);
    Some(Marker {
        indent: lead,
        number,
        content: lead + width + gap,
    })
}

fn starts_block(line: &str) -> bool {
    let trimmed = line.trim_start();
    heading(line).is_some()
        || trimmed.starts_with("```")
        || trimmed.starts_with('>')
        || list_marker(line).is_some()
}

fn continues(item: &[String], line: &str) -> bool {
    item.last().is_some_and(|last| !last.trim().is_empty()) && !starts_block(line)
}

pub(super) fn take_list(lines: &[&str], start: usize, depth: usize) -> Option<(List, usize)> {
    let first = list_marker(lines.get(start)?)?;
    let mut items: Vec<Vec<String>> = Vec::new();
    let mut column = first.content;
    let mut i = start;
    while let Some(&line) = lines.get(i) {
        if let Some(marker) = list_marker(line).filter(|m| m.indent < column) {
            if marker.number.is_some() != first.number.is_some() || marker.indent > first.indent + 3
            {
                break;
            }
            column = marker.content;
            items.push(vec![
                line.get(column.min(line.len())..)
                    .unwrap_or_default()
                    .to_string(),
            ]);
        } else if line.trim().is_empty() {
            let next = lines.get(i + 1..)?.iter().find(|l| !l.trim().is_empty());
            if next.is_none_or(|next| indent(next) < column) {
                break;
            }
            items.last_mut()?.push(String::new());
        } else if indent(line) >= column || continues(items.last()?, line) {
            let body = if indent(line) >= column {
                dedent(line, column)
            } else {
                line.trim()
            };
            items.last_mut()?.push(body.to_string());
        } else {
            break;
        }
        i += 1;
    }
    let items = items.iter().map(|item| list_item(item, depth)).collect();
    Some((
        List {
            start: first.number,
            items,
        },
        i - start,
    ))
}

fn quote_line(line: &str) -> Option<&str> {
    if indent(line) > 3 {
        return None;
    }
    let rest = line.trim_start().strip_prefix('>')?;
    Some(rest.strip_prefix(' ').unwrap_or(rest))
}

pub(super) fn take_quote(lines: &[&str], start: usize) -> Option<(String, usize)> {
    quote_line(lines.get(start)?)?;
    let mut inner: Vec<&str> = Vec::new();
    let mut i = start;
    while let Some(&line) = lines.get(i) {
        if let Some(rest) = quote_line(line) {
            inner.push(rest);
        } else if !line.trim().is_empty()
            && inner.last().is_some_and(|last| !last.trim().is_empty())
            && !starts_block(line)
        {
            inner.push(line.trim());
        } else {
            break;
        }
        i += 1;
    }
    Some((inner.join("\n"), i - start))
}

pub(super) fn rule(line: &str) -> bool {
    let marks: Vec<char> = line.chars().filter(|ch| *ch != ' ').collect();
    indent(line) < 4
        && marks.len() >= 3
        && matches!(marks.first(), Some('-' | '*' | '_'))
        && marks.iter().all(|ch| Some(ch) == marks.first())
}

pub(super) fn heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &line[hashes..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let body = rest.trim();
    let closed = body.trim_end_matches('#');
    let body = if closed.is_empty() || closed.ends_with(' ') {
        closed.trim_end()
    } else {
        body
    };
    Some((u8::try_from(hashes).unwrap_or(6), body))
}

pub(super) fn image_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("![")?;
    let close = rest.find("](")?;
    let src = rest[close + 2..].strip_suffix(')')?;
    Some((&rest[..close], src))
}

pub(super) fn take_table(lines: &[&str], start: usize) -> Option<(Vec<Vec<String>>, usize)> {
    let header = pipe_row(lines.get(start).copied()?)?;
    let sep = lines.get(start + 1).copied()?;
    if !is_table_sep(sep) {
        return None;
    }
    let mut rows = vec![header];
    let mut i = start + 2;
    while let Some(row) = lines.get(i).copied().and_then(pipe_row) {
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
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let cells: Vec<String> = inner
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect();
    (!cells.is_empty()).then_some(cells)
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
