use super::flow::{Dir, Edge, Graph, Node, Shape};

const MAX_EDGES: usize = 1024;
const MAX_STATEMENT_EDGES: usize = 256;

struct Link {
    label: String,
    dashed: bool,
    arrow: bool,
}

type Detail = Option<(Vec<String>, Shape)>;

enum Part {
    Nodes(Vec<(String, Detail)>),
    Link(Link),
}

pub(super) fn parse(source: &str) -> Option<Graph> {
    let mut lines = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("%%"));
    let (state, dir) = header(lines.next()?)?;
    let mut graph = Graph {
        nodes: Vec::new(),
        edges: Vec::new(),
        dir,
        state,
    };
    for line in lines {
        if let Some((id, label)) = alias(line).filter(|_| state) {
            graph.node(id, Some((vec![label.to_string()], Shape::Box)));
        } else if !skipped(line) {
            graph.statement(line);
        }
    }
    Some(graph)
}

fn header(line: &str) -> Option<(bool, Dir)> {
    let mut words = line.split_whitespace();
    let kind = words.next()?;
    let dir = match words.next().map(|word| word.trim_end_matches(';')) {
        Some("LR") => Dir::Right,
        Some("RL") => Dir::Left,
        Some("BT") => Dir::Up,
        _ => Dir::Down,
    };
    match kind {
        "graph" | "flowchart" => Some((false, dir)),
        "stateDiagram" | "stateDiagram-v2" => Some((true, Dir::Down)),
        _ => None,
    }
}

fn skipped(line: &str) -> bool {
    let head = line.split_whitespace().next().unwrap_or("");
    line.starts_with('}')
        || matches!(
            head,
            "click"
                | "style"
                | "classDef"
                | "class"
                | "linkStyle"
                | "subgraph"
                | "end"
                | "direction"
                | "state"
                | "note"
                | "accTitle:"
                | "accDescr:"
        )
}

fn alias(line: &str) -> Option<(&str, &str)> {
    let (label, id) = line.strip_prefix("state ")?.trim().split_once(" as ")?;
    Some((id.trim(), label.trim().trim_matches('"')))
}

impl Graph {
    fn statement(&mut self, line: &str) {
        let (line, colon) = match (self.state, line.split_once(':')) {
            (true, Some((head, tail))) => (head.trim(), Some(tail.trim().to_string())),
            _ => (line, None),
        };
        let mut previous: Option<Vec<usize>> = None;
        let mut pending: Option<Link> = None;
        for part in tokens(line) {
            match part {
                Part::Link(found) => pending = Some(found),
                Part::Nodes(nodes) => {
                    let first = previous.is_none();
                    let ids: Vec<usize> = nodes
                        .into_iter()
                        .map(|(id, detail)| self.node_or_terminal(&id, detail, first))
                        .collect();
                    if let (Some(from), Some(found)) = (&previous, pending.take()) {
                        let pairs = from
                            .iter()
                            .flat_map(|source| ids.iter().map(move |target| (source, target)));
                        let room = MAX_EDGES
                            .saturating_sub(self.edges.len())
                            .min(MAX_STATEMENT_EDGES);
                        for (source, target) in pairs.take(room) {
                            self.edges.push(Edge {
                                from: *source,
                                to: *target,
                                label: colon.clone().unwrap_or_else(|| found.label.clone()),
                                dashed: found.dashed,
                                arrow: found.arrow,
                            });
                        }
                    }
                    previous = Some(ids);
                }
            }
        }
    }

    fn node_or_terminal(&mut self, id: &str, detail: Detail, first: bool) -> usize {
        if id == "[*]" {
            let (key, glyph) = if first {
                ("[*]start", "●")
            } else {
                ("[*]end", "◉")
            };
            return self.node(key, Some((vec![glyph.to_string()], Shape::Terminal)));
        }
        self.node(id, detail)
    }

    fn node(&mut self, id: &str, detail: Detail) -> usize {
        let index = self
            .nodes
            .iter()
            .position(|node| node.id == id)
            .unwrap_or_else(|| {
                self.nodes.push(Node {
                    id: id.to_string(),
                    label: vec![id.to_string()],
                    shape: Shape::Box,
                    dummy: false,
                    dashed: false,
                });
                self.nodes.len().saturating_sub(1)
            });
        if let (Some((label, shape)), Some(node)) = (detail, self.nodes.get_mut(index)) {
            node.label = label;
            node.shape = shape;
        }
        index
    }
}

fn tokens(line: &str) -> Vec<Part> {
    let chars: Vec<char> = line.chars().collect();
    let mut at = 0;
    let mut parts = Vec::new();
    let mut nodes: Vec<(String, Detail)> = Vec::new();
    while at < chars.len() {
        let ch = chars.get(at).copied().unwrap_or(' ');
        if ch.is_whitespace() || ch == ';' || ch == '&' {
            at = at.saturating_add(1);
            continue;
        }
        if link_starts(&chars, at) {
            if !nodes.is_empty() {
                parts.push(Part::Nodes(std::mem::take(&mut nodes)));
            }
            let (found, next) = link(&chars, at);
            parts.push(Part::Link(found));
            at = next.max(at.saturating_add(1));
            continue;
        }
        let (node, next) = node_token(&chars, at);
        if next == at {
            at = at.saturating_add(1);
            continue;
        }
        nodes.push(node);
        at = next;
    }
    if !nodes.is_empty() {
        parts.push(Part::Nodes(nodes));
    }
    parts
}

fn link_starts(chars: &[char], at: usize) -> bool {
    let first = chars.get(at).copied().unwrap_or(' ');
    let second = chars.get(at.saturating_add(1)).copied().unwrap_or(' ');
    match first {
        '-' | '=' => matches!(second, '-' | '=' | '.' | '>'),
        '<' => matches!(second, '-' | '='),
        _ => false,
    }
}

fn scan(chars: &[char], at: usize, keep: impl Fn(char) -> bool) -> (String, usize) {
    let text: String = chars
        .iter()
        .skip(at)
        .copied()
        .take_while(|ch| keep(*ch))
        .collect();
    let end = at.saturating_add(text.chars().count());
    (text, end)
}

fn run(chars: &[char], at: usize, set: &str) -> (String, usize) {
    scan(chars, at, |ch| set.contains(ch))
}

fn link(chars: &[char], at: usize) -> (Link, usize) {
    let (head, after) = run(chars, at, "-=.<>xo");
    let mut cursor = scan(chars, after, char::is_whitespace).1;
    let mut label = String::new();
    let mut tail = String::new();
    if !head.contains('>') && !head.contains('x') && !head.contains('o') {
        if let Some((text, next)) = inline_label(chars, cursor) {
            label = text;
            let (found, after_tail) = run(chars, next, "-=.>xo");
            tail = found;
            cursor = after_tail;
        }
    } else if chars.get(cursor) == Some(&'|') {
        let (text, end) = scan(chars, cursor.saturating_add(1), |ch| ch != '|');
        label = text;
        cursor = end.saturating_add(1);
    }
    let both = format!("{head}{tail}");
    let link = Link {
        label: label.trim().to_string(),
        dashed: both.contains('.'),
        arrow: both.contains('>') || both.contains('x') || both.contains('o'),
    };
    (link, cursor)
}

fn inline_label(chars: &[char], at: usize) -> Option<(String, usize)> {
    let mut end = at;
    let mut text = String::new();
    while end < chars.len() {
        let space = chars.get(end).is_some_and(|ch| ch.is_whitespace());
        let next = end.saturating_add(1);
        let starts = chars
            .get(next)
            .is_some_and(|ch| matches!(ch, '-' | '=' | '.'))
            && chars
                .get(next.saturating_add(1))
                .is_some_and(|ch| matches!(ch, '-' | '=' | '.' | '>'));
        if space && starts {
            return (!text.trim().is_empty()).then_some((text, next));
        }
        text.push(chars.get(end).copied().unwrap_or(' '));
        end = next;
    }
    None
}

fn node_token(chars: &[char], at: usize) -> ((String, Detail), usize) {
    if chars.get(at..at.saturating_add(3)) == Some(&['[', '*', ']']) {
        return (("[*]".to_string(), None), at.saturating_add(3));
    }
    let (id, end) = scan(chars, at, |ch| ch.is_alphanumeric() || ch == '_');
    if id.is_empty() {
        return ((id, None), at);
    }
    let Some(open) = chars
        .get(end)
        .copied()
        .filter(|ch| matches!(ch, '[' | '(' | '{' | '>'))
    else {
        return ((id, None), end);
    };
    let close = group_end(chars, end);
    let raw: String = chars.get(end..close).unwrap_or_default().iter().collect();
    let detail = Some((label_lines(&raw), shape_of(&raw, open)));
    ((id, detail), close)
}

fn group_end(chars: &[char], start: usize) -> usize {
    let mut depth = 0_usize;
    let mut end = start;
    let mut quoted = false;
    while let Some(ch) = chars.get(end).copied() {
        end = end.saturating_add(1);
        if ch == '"' {
            quoted = !quoted;
            continue;
        }
        if quoted {
            continue;
        }
        match ch {
            '[' | '(' | '{' => depth = depth.saturating_add(1),
            '>' if end == start.saturating_add(1) => depth = depth.saturating_add(1),
            ']' | ')' | '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return end;
                }
            }
            _ => {}
        }
    }
    end
}

fn shape_of(raw: &str, open: char) -> Shape {
    let paired = [
        ("((", Shape::Circle),
        ("([", Shape::Stadium),
        ("[[", Shape::Subroutine),
        ("[(", Shape::Cylinder),
        ("{{", Shape::Hexagon),
    ];
    let single = match open {
        '{' => Shape::Diamond,
        '(' => Shape::Round,
        '>' => Shape::Flag,
        _ => Shape::Box,
    };
    paired
        .into_iter()
        .find_map(|(prefix, shape)| raw.starts_with(prefix).then_some(shape))
        .unwrap_or(single)
}

fn label_lines(raw: &str) -> Vec<String> {
    let inner = raw
        .trim_start_matches(['[', '(', '{', '>', '/', '\\'])
        .trim_end_matches([']', ')', '}', '/', '\\'])
        .trim()
        .trim_matches('"');
    super::label::breaks(inner)
}
