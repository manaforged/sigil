use super::flow::{Graph, Shape};
use super::flow_parse::{alias, skipped};

pub(super) fn fill<'a>(graph: &mut Graph, lines: impl Iterator<Item = &'a str>) {
    let mut in_note = false;
    let mut scopes: Vec<String> = Vec::new();
    for line in lines {
        if in_note {
            in_note = !line.starts_with("end note");
            continue;
        }
        if line.starts_with("note ") {
            in_note = !line.contains(':');
            continue;
        }
        if let Some(name) = composite(line) {
            graph.node(name, None);
            scopes.push(name.to_string());
            continue;
        }
        if line.starts_with('}') {
            scopes.pop();
            continue;
        }
        if let Some((id, label)) = alias(line) {
            graph.node(id, Some((vec![label.to_string()], Shape::Box)));
            continue;
        }
        if skipped(line) {
            continue;
        }
        match description(line) {
            Some((id, text)) => graph.describe(id, text),
            None => graph.statement(line, scopes.last().map(String::as_str)),
        }
    }
}

fn composite(line: &str) -> Option<&str> {
    let body = line.strip_prefix("state ")?.strip_suffix('{')?.trim();
    let name = body.rsplit_once(" as ").map_or(body, |(_, id)| id.trim());
    (!name.is_empty() && !name.contains(char::is_whitespace)).then_some(name)
}

fn description(line: &str) -> Option<(&str, &str)> {
    if line.contains("--") {
        return None;
    }
    let (id, text) = line.split_once(':')?;
    let id = id.trim();
    let named = !id.is_empty()
        && id
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '-' | '.'));
    named.then(|| (id, text.trim()))
}
