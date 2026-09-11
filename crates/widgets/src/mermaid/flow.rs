use std::collections::{BTreeSet, HashMap};

use super::canvas::{Canvas, DOWN, LEFT, RIGHT, UP};

const GAP_ROWS: usize = 3;
const GAP_COLS: usize = 5;
const SPACING: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Box,
    Round,
    Stadium,
    Circle,
    Diamond,
    Hexagon,
    Cylinder,
    Subroutine,
    Flag,
    Terminal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir {
    Down,
    Up,
    Right,
    Left,
}

impl Dir {
    const fn vertical(self) -> bool {
        matches!(self, Self::Down | Self::Up)
    }

    const fn forward(self) -> bool {
        matches!(self, Self::Down | Self::Right)
    }
}

struct Node {
    id: String,
    label: Vec<String>,
    shape: Shape,
    dummy: bool,
    dashed: bool,
}

struct Edge {
    from: usize,
    to: usize,
    label: String,
    dashed: bool,
    arrow: bool,
}

struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    dir: Dir,
    state: bool,
}

#[derive(Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

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

pub(super) fn draw(source: &str) -> Option<Vec<String>> {
    let graph = parse(source)?;
    if graph.nodes.is_empty() {
        return None;
    }
    Some(render(graph))
}

fn parse(source: &str) -> Option<Graph> {
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
        if let Some((id, label)) = alias(line, state) {
            graph.node(&id, Some((vec![label], Shape::Box)));
            continue;
        }
        if skipped(line) {
            continue;
        }
        statement(&mut graph, line);
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

fn alias(line: &str, state: bool) -> Option<(String, String)> {
    if !state {
        return None;
    }
    let rest = line.strip_prefix("state ")?.trim();
    let (label, id) = rest.split_once(" as ")?;
    Some((
        id.trim().to_string(),
        label.trim().trim_matches('"').to_string(),
    ))
}

fn statement(graph: &mut Graph, line: &str) {
    let (line, colon) = match (graph.state, line.split_once(':')) {
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
                    .map(|(id, detail)| graph.node_or_terminal(&id, detail, first))
                    .collect();
                if let (Some(from), Some(found)) = (&previous, pending.take()) {
                    for source in from {
                        for target in &ids {
                            graph.edges.push(Edge {
                                from: *source,
                                to: *target,
                                label: colon.clone().unwrap_or_else(|| found.label.clone()),
                                dashed: found.dashed,
                                arrow: found.arrow,
                            });
                        }
                    }
                }
                previous = Some(ids);
            }
        }
    }
}

impl Graph {
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

fn run(chars: &[char], at: usize, set: &str) -> (String, usize) {
    let mut end = at;
    let mut text = String::new();
    while let Some(ch) = chars.get(end).copied().filter(|ch| set.contains(*ch)) {
        text.push(ch);
        end = end.saturating_add(1);
    }
    (text, end)
}

fn skip_spaces(chars: &[char], mut at: usize) -> usize {
    while chars.get(at).is_some_and(|ch| ch.is_whitespace()) {
        at = at.saturating_add(1);
    }
    at
}

fn link(chars: &[char], at: usize) -> (Link, usize) {
    let (head, after) = run(chars, at, "-=.<>xo");
    let mut cursor = skip_spaces(chars, after);
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
        let mut end = cursor.saturating_add(1);
        while let Some(ch) = chars.get(end).copied().filter(|ch| *ch != '|') {
            label.push(ch);
            end = end.saturating_add(1);
        }
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
    let mut end = at;
    let mut id = String::new();
    while let Some(ch) = chars
        .get(end)
        .copied()
        .filter(|ch| ch.is_alphanumeric() || *ch == '_')
    {
        id.push(ch);
        end = end.saturating_add(1);
    }
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
    if raw.starts_with("((") {
        Shape::Circle
    } else if raw.starts_with("([") {
        Shape::Stadium
    } else if raw.starts_with("[[") {
        Shape::Subroutine
    } else if raw.starts_with("[(") {
        Shape::Cylinder
    } else if raw.starts_with("{{") {
        Shape::Hexagon
    } else {
        match open {
            '{' => Shape::Diamond,
            '(' => Shape::Round,
            '>' => Shape::Flag,
            _ => Shape::Box,
        }
    }
}

fn label_lines(raw: &str) -> Vec<String> {
    let inner = raw
        .trim_start_matches(['[', '(', '{', '>', '/', '\\'])
        .trim_end_matches([']', ')', '}', '/', '\\'])
        .trim()
        .trim_matches('"');
    inner
        .replace("<br/>", "\n")
        .replace("<br>", "\n")
        .lines()
        .map(|line| line.trim().to_string())
        .collect()
}

fn render(graph: Graph) -> Vec<String> {
    let ranks = rank(&graph);
    let (graph, ranks) = expand(graph, ranks);
    let layers = order(&graph, &ranks);
    let rects = place(&graph, &ranks, &layers);
    paint(&graph, &rects)
}

fn rank(graph: &Graph) -> Vec<usize> {
    let back = back_edges(graph);
    let mut rank = vec![0_usize; graph.nodes.len()];
    for _ in 0..=graph.nodes.len() {
        let mut changed = false;
        for (index, edge) in graph.edges.iter().enumerate() {
            if back.contains(&index) || edge.from == edge.to {
                continue;
            }
            let want = rank.get(edge.from).copied().unwrap_or(0).saturating_add(1);
            if let Some(slot) = rank.get_mut(edge.to)
                && *slot < want
            {
                slot.clone_from(&want);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    rank
}

fn back_edges(graph: &Graph) -> BTreeSet<usize> {
    let mut colour = vec![0_u8; graph.nodes.len()];
    let mut back = BTreeSet::new();
    for start in 0..graph.nodes.len() {
        if colour.get(start) == Some(&0) {
            visit(graph, start, &mut colour, &mut back);
        }
    }
    back
}

fn visit(graph: &Graph, node: usize, colour: &mut [u8], back: &mut BTreeSet<usize>) {
    if let Some(slot) = colour.get_mut(node) {
        slot.clone_from(&1);
    }
    for (index, edge) in graph.edges.iter().enumerate() {
        if edge.from != node {
            continue;
        }
        match colour.get(edge.to).copied() {
            Some(1) => drop(back.insert(index)),
            Some(0) => visit(graph, edge.to, colour, back),
            _ => {}
        }
    }
    if let Some(slot) = colour.get_mut(node) {
        slot.clone_from(&2);
    }
}

fn expand(mut graph: Graph, mut ranks: Vec<usize>) -> (Graph, Vec<usize>) {
    let mut edges = Vec::new();
    for edge in std::mem::take(&mut graph.edges) {
        let from_rank = ranks.get(edge.from).copied().unwrap_or(0);
        let to_rank = ranks.get(edge.to).copied().unwrap_or(0);
        if to_rank <= from_rank.saturating_add(1) {
            edges.push(edge);
            continue;
        }
        let mut previous = edge.from;
        for rank in from_rank.saturating_add(1)..to_rank {
            graph.nodes.push(Node {
                id: String::new(),
                label: Vec::new(),
                shape: Shape::Box,
                dummy: true,
                dashed: edge.dashed,
            });
            ranks.push(rank);
            let dummy = graph.nodes.len().saturating_sub(1);
            edges.push(Edge {
                from: previous,
                to: dummy,
                label: if previous == edge.from {
                    edge.label.clone()
                } else {
                    String::new()
                },
                dashed: edge.dashed,
                arrow: false,
            });
            previous = dummy;
        }
        edges.push(Edge {
            from: previous,
            to: edge.to,
            label: String::new(),
            dashed: edge.dashed,
            arrow: edge.arrow,
        });
    }
    graph.edges = edges;
    (graph, ranks)
}

fn order(graph: &Graph, ranks: &[usize]) -> Vec<Vec<usize>> {
    let depth = ranks.iter().max().map_or(0, |rank| rank.saturating_add(1));
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); depth];
    for (node, rank) in ranks.iter().enumerate() {
        if let Some(layer) = layers.get_mut(*rank) {
            layer.push(node);
        }
    }
    for _ in 0..3 {
        for rank in 1..depth {
            sweep(graph, &mut layers, rank, true);
        }
        for rank in (0..depth.saturating_sub(1)).rev() {
            sweep(graph, &mut layers, rank, false);
        }
    }
    layers
}

fn sweep(graph: &Graph, layers: &mut [Vec<usize>], rank: usize, down: bool) {
    let neighbour = if down {
        rank.saturating_sub(1)
    } else {
        rank.saturating_add(1)
    };
    let positions: HashMap<usize, usize> = layers
        .get(neighbour)
        .map(|layer| {
            layer
                .iter()
                .enumerate()
                .map(|(index, node)| (*node, index))
                .collect()
        })
        .unwrap_or_default();
    let Some(layer) = layers.get(rank).cloned() else {
        return;
    };
    let mut keyed: Vec<(usize, usize)> = layer
        .iter()
        .enumerate()
        .map(|(index, node)| (barycenter(graph, *node, &positions, down, index), *node))
        .collect();
    keyed.sort_by_key(|(key, _)| *key);
    if let Some(slot) = layers.get_mut(rank) {
        slot.clone_from(&keyed.into_iter().map(|(_, node)| node).collect());
    }
}

fn barycenter(
    graph: &Graph,
    node: usize,
    positions: &HashMap<usize, usize>,
    down: bool,
    index: usize,
) -> usize {
    let mut sum = 0_usize;
    let mut count = 0_usize;
    for edge in &graph.edges {
        let other = match (down, edge.to == node, edge.from == node) {
            (true, true, _) => edge.from,
            (false, _, true) => edge.to,
            _ => continue,
        };
        if let Some(position) = positions.get(&other) {
            sum = sum.saturating_add(position.saturating_mul(1000));
            count = count.saturating_add(1);
        }
    }
    sum.checked_div(count)
        .unwrap_or_else(|| index.saturating_mul(1000))
}

fn size(node: &Node) -> (usize, usize) {
    if node.dummy {
        return (1, 1);
    }
    let text = node
        .label
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let extra = match node.shape {
        Shape::Circle | Shape::Stadium | Shape::Subroutine | Shape::Flag => 2,
        _ => 0,
    };
    (
        text.saturating_add(extra).saturating_add(4),
        node.label.len().max(1).saturating_add(2),
    )
}

fn place(graph: &Graph, ranks: &[usize], layers: &[Vec<usize>]) -> Vec<Rect> {
    let sizes: Vec<(usize, usize)> = graph.nodes.iter().map(size).collect();
    let vertical = graph.dir.vertical();
    let along = |node: usize| {
        let (w, h) = sizes.get(node).copied().unwrap_or((1, 1));
        if vertical { h } else { w }
    };
    let across = |node: usize| {
        let (w, h) = sizes.get(node).copied().unwrap_or((1, 1));
        if vertical { w } else { h }
    };
    let rank_size: Vec<usize> = layers
        .iter()
        .map(|layer| layer.iter().map(|node| along(*node)).max().unwrap_or(1))
        .collect();
    let gap = if vertical { GAP_ROWS } else { GAP_COLS };
    let offsets = offsets(layers, &rank_size, gap, graph.dir.forward());
    let positions = positions(graph, layers, &across);
    graph
        .nodes
        .iter()
        .enumerate()
        .map(|(node, entry)| {
            let (w, h) = sizes.get(node).copied().unwrap_or((1, 1));
            let rank = ranks.get(node).copied().unwrap_or(0);
            let offset = offsets.get(rank).copied().unwrap_or(0);
            let span = rank_size.get(rank).copied().unwrap_or(1);
            let at = positions.get(node).copied().unwrap_or(0);
            if vertical {
                Rect {
                    x: at,
                    y: offset,
                    w,
                    h: if entry.dummy { span } else { h },
                }
            } else {
                Rect {
                    x: offset,
                    y: at,
                    w: if entry.dummy { span } else { w },
                    h,
                }
            }
        })
        .collect()
}

fn offsets(layers: &[Vec<usize>], rank_size: &[usize], gap: usize, forward: bool) -> Vec<usize> {
    let mut offsets = vec![0_usize; layers.len()];
    let mut cursor = 0_usize;
    let order: Vec<usize> = if forward {
        (0..layers.len()).collect()
    } else {
        (0..layers.len()).rev().collect()
    };
    for rank in order {
        if let Some(slot) = offsets.get_mut(rank) {
            slot.clone_from(&cursor);
        }
        cursor = cursor
            .saturating_add(rank_size.get(rank).copied().unwrap_or(1))
            .saturating_add(gap);
    }
    offsets
}

fn positions(graph: &Graph, layers: &[Vec<usize>], across: &dyn Fn(usize) -> usize) -> Vec<usize> {
    let mut positions = vec![0_usize; graph.nodes.len()];
    for layer in layers {
        let mut cursor = 0_usize;
        for node in layer {
            let extent = across(*node);
            let desired = parents(graph, *node, &positions, across)
                .map_or(cursor, |centre| centre.saturating_sub(extent / 2));
            let at = desired.max(cursor);
            if let Some(slot) = positions.get_mut(*node) {
                slot.clone_from(&at);
            }
            cursor = at.saturating_add(extent).saturating_add(SPACING);
        }
    }
    positions
}

fn parents(
    graph: &Graph,
    node: usize,
    positions: &[usize],
    across: &dyn Fn(usize) -> usize,
) -> Option<usize> {
    let centres: Vec<usize> = graph
        .edges
        .iter()
        .filter(|edge| edge.to == node && edge.from != node)
        .filter_map(|edge| {
            let at = positions.get(edge.from)?;
            Some(at.saturating_add(across(edge.from) / 2))
        })
        .collect();
    if centres.is_empty() {
        return None;
    }
    Some(centres.iter().sum::<usize>() / centres.len())
}

fn paint(graph: &Graph, rects: &[Rect]) -> Vec<String> {
    let (cols, rows, extent, lanes) = sizes(graph, rects);
    let mut canvas = Canvas::new(cols, rows);
    for (node, rect) in graph.nodes.iter().zip(rects) {
        if node.dummy {
            through(&mut canvas, *rect, graph.dir, node.dashed);
        } else {
            shape(&mut canvas, node, *rect);
        }
    }
    for (edge, lane) in graph.edges.iter().zip(&lanes) {
        let (Some(from), Some(to)) = (rects.get(edge.from), rects.get(edge.to)) else {
            continue;
        };
        match lane {
            Some(lane) => {
                let at = extent
                    .saturating_add(1)
                    .saturating_add(lane.saturating_mul(2));
                around(&mut canvas, edge, (*from, *to), graph.dir, at);
            }
            None => route(&mut canvas, edge, *from, *to, graph.dir),
        }
    }
    canvas.lines()
}

fn sizes(graph: &Graph, rects: &[Rect]) -> (usize, usize, usize, Vec<Option<usize>>) {
    let margin = graph
        .edges
        .iter()
        .map(|edge| edge.label.chars().count())
        .max()
        .unwrap_or(0)
        .saturating_add(2);
    let cols = rects
        .iter()
        .map(|rect| rect.x.saturating_add(rect.w))
        .max()
        .unwrap_or(0)
        .saturating_add(margin);
    let rows = rects
        .iter()
        .map(|rect| rect.y.saturating_add(rect.h))
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let lanes = lanes(graph, rects);
    let extent = rects
        .iter()
        .map(|rect| {
            if graph.dir.vertical() {
                rect.x.saturating_add(rect.w)
            } else {
                rect.y.saturating_add(rect.h)
            }
        })
        .max()
        .unwrap_or(0);
    let back = match lanes.iter().flatten().count() {
        0 => 0,
        count => count.saturating_mul(2).saturating_add(2),
    };
    let (cols, rows) = if graph.dir.vertical() {
        (cols.saturating_add(back), rows)
    } else {
        (cols, rows.saturating_add(back))
    };
    (cols, rows, extent, lanes)
}

fn lanes(graph: &Graph, rects: &[Rect]) -> Vec<Option<usize>> {
    let mut next = 0_usize;
    let pen = Pen {
        dir: graph.dir,
        dashed: false,
    };
    graph
        .edges
        .iter()
        .map(|edge| {
            let (Some(from), Some(to)) = (rects.get(edge.from), rects.get(edge.to)) else {
                return None;
            };
            let (pp, _) = pen.split(exit(*from, graph.dir));
            let (qp, _) = pen.split(entry(*to, graph.dir));
            if (qp > pp) == graph.dir.forward() && qp != pp {
                return None;
            }
            let lane = next;
            next = next.saturating_add(1);
            Some(lane)
        })
        .collect()
}

fn around(canvas: &mut Canvas, edge: &Edge, ends: (Rect, Rect), dir: Dir, lane: usize) {
    let (from, to) = ends;
    let pen = Pen {
        dir,
        dashed: edge.dashed,
    };
    let (sp, ss) = pen.split(side(from, dir));
    let (tp, ts) = pen.split(side(to, dir));
    let toward = if dir.vertical() { LEFT } else { UP };
    let (first, second) = if tp < sp {
        (pen.back(), pen.ahead())
    } else {
        (pen.ahead(), pen.back())
    };
    if let Some((low, high)) = span(ss, lane) {
        pen.secondary(canvas, sp, low, high);
    }
    let (x1, y1) = pen.point(sp, lane);
    canvas.corner(x1, y1, toward | first);
    if let Some((low, high)) = between(sp, tp) {
        pen.primary(canvas, lane, low, high);
    }
    let (x2, y2) = pen.point(tp, lane);
    canvas.corner(x2, y2, toward | second);
    if let Some((low, high)) = between(ts, lane) {
        pen.secondary(canvas, tp, low, high);
    }
    let head = match (edge.arrow, dir.vertical()) {
        (true, true) => '◀',
        (true, false) => '▲',
        (false, _) => pen.glyphs().1,
    };
    let (x3, y3) = pen.point(tp, ts);
    canvas.set(x3, y3, head);
    if !edge.label.is_empty() {
        let middle = sp.min(tp).saturating_add(sp.abs_diff(tp) / 2);
        let (x4, y4) = pen.point(middle, lane.saturating_add(1));
        canvas.text(x4, y4, &edge.label);
    }
}

const fn side(rect: Rect, dir: Dir) -> (usize, usize) {
    if dir.vertical() {
        (rect.x + rect.w, rect.y + rect.h / 2)
    } else {
        (rect.x + rect.w / 2, rect.y + rect.h)
    }
}

fn through(canvas: &mut Canvas, rect: Rect, dir: Dir, dashed: bool) {
    let pen = Pen { dir, dashed };
    let (glyph, _) = pen.glyphs();
    if dir.vertical() {
        canvas.vline(
            rect.x,
            rect.y,
            rect.y.saturating_add(rect.h).saturating_sub(1),
            glyph,
        );
    } else {
        canvas.hline(
            rect.x,
            rect.x.saturating_add(rect.w).saturating_sub(1),
            rect.y,
            glyph,
        );
    }
}

fn shape(canvas: &mut Canvas, node: &Node, rect: Rect) {
    let round = matches!(
        node.shape,
        Shape::Round | Shape::Stadium | Shape::Cylinder | Shape::Circle | Shape::Terminal
    );
    canvas.frame(rect.x, rect.y, (rect.w, rect.h), round);
    if matches!(node.shape, Shape::Diamond | Shape::Hexagon) {
        let right = rect.x.saturating_add(rect.w).saturating_sub(1);
        let bottom = rect.y.saturating_add(rect.h).saturating_sub(1);
        canvas.set(rect.x, rect.y, '╱');
        canvas.set(right, rect.y, '╲');
        canvas.set(rect.x, bottom, '╲');
        canvas.set(right, bottom, '╱');
    }
    let inner = rect.w.saturating_sub(2);
    for (index, line) in node.label.iter().enumerate() {
        let text = match node.shape {
            Shape::Circle | Shape::Stadium => format!("({line})"),
            Shape::Subroutine => format!("[{line}]"),
            Shape::Flag => format!("{line} ▷"),
            _ => line.clone(),
        };
        let width = text.chars().count();
        let x = rect
            .x
            .saturating_add(1)
            .saturating_add(inner.saturating_sub(width) / 2);
        canvas.text(x, rect.y.saturating_add(1).saturating_add(index), &text);
    }
}

struct Pen {
    dir: Dir,
    dashed: bool,
}

impl Pen {
    const fn split(&self, point: (usize, usize)) -> (usize, usize) {
        if self.dir.vertical() {
            (point.1, point.0)
        } else {
            (point.0, point.1)
        }
    }

    const fn point(&self, primary: usize, secondary: usize) -> (usize, usize) {
        if self.dir.vertical() {
            (secondary, primary)
        } else {
            (primary, secondary)
        }
    }

    const fn glyphs(&self) -> (char, char) {
        match (self.dir.vertical(), self.dashed) {
            (true, false) => ('│', '─'),
            (true, true) => ('┆', '┄'),
            (false, false) => ('─', '│'),
            (false, true) => ('┄', '┆'),
        }
    }

    fn primary(&self, canvas: &mut Canvas, secondary: usize, from: usize, to: usize) {
        let (glyph, _) = self.glyphs();
        if self.dir.vertical() {
            canvas.vline(secondary, from, to, glyph);
        } else {
            canvas.hline(from, to, secondary, glyph);
        }
    }

    fn secondary(&self, canvas: &mut Canvas, primary: usize, from: usize, to: usize) {
        let (_, glyph) = self.glyphs();
        if self.dir.vertical() {
            canvas.hline(from, to, primary, glyph);
        } else {
            canvas.vline(primary, from, to, glyph);
        }
    }

    const fn back(&self) -> u8 {
        match self.dir {
            Dir::Down => UP,
            Dir::Up => DOWN,
            Dir::Right => LEFT,
            Dir::Left => RIGHT,
        }
    }

    const fn ahead(&self) -> u8 {
        match self.dir {
            Dir::Down => DOWN,
            Dir::Up => UP,
            Dir::Right => RIGHT,
            Dir::Left => LEFT,
        }
    }

    const fn sideways(&self, from: usize, to: usize) -> (u8, u8) {
        match (self.dir.vertical(), to > from) {
            (true, true) => (RIGHT, LEFT),
            (true, false) => (LEFT, RIGHT),
            (false, true) => (DOWN, UP),
            (false, false) => (UP, DOWN),
        }
    }

    const fn arrow(&self) -> char {
        match self.dir {
            Dir::Down => '▼',
            Dir::Up => '▲',
            Dir::Right => '▶',
            Dir::Left => '◀',
        }
    }
}

const fn exit(rect: Rect, dir: Dir) -> (usize, usize) {
    match dir {
        Dir::Down => (rect.x + rect.w / 2, rect.y + rect.h),
        Dir::Up => (rect.x + rect.w / 2, rect.y.saturating_sub(1)),
        Dir::Right => (rect.x + rect.w, rect.y + rect.h / 2),
        Dir::Left => (rect.x.saturating_sub(1), rect.y + rect.h / 2),
    }
}

const fn entry(rect: Rect, dir: Dir) -> (usize, usize) {
    match dir {
        Dir::Down => (rect.x + rect.w / 2, rect.y.saturating_sub(1)),
        Dir::Up => (rect.x + rect.w / 2, rect.y + rect.h),
        Dir::Right => (rect.x.saturating_sub(1), rect.y + rect.h / 2),
        Dir::Left => (rect.x + rect.w, rect.y + rect.h / 2),
    }
}

fn span(from: usize, to: usize) -> Option<(usize, usize)> {
    match from.cmp(&to) {
        std::cmp::Ordering::Less => Some((from, to.saturating_sub(1))),
        std::cmp::Ordering::Greater => Some((to.saturating_add(1), from)),
        std::cmp::Ordering::Equal => None,
    }
}

fn between(from: usize, to: usize) -> Option<(usize, usize)> {
    let (low, high) = (from.min(to), from.max(to));
    (high.saturating_sub(low) > 1).then(|| (low.saturating_add(1), high.saturating_sub(1)))
}

fn route(canvas: &mut Canvas, edge: &Edge, from: Rect, to: Rect, dir: Dir) {
    let pen = Pen {
        dir,
        dashed: edge.dashed,
    };
    let (pp, ps) = pen.split(exit(from, dir));
    let (qp, qs) = pen.split(entry(to, dir));
    let head = if edge.arrow {
        pen.arrow()
    } else {
        pen.glyphs().0
    };
    if ps == qs {
        if let Some((low, high)) = span(pp, qp) {
            pen.primary(canvas, ps, low, high);
        }
        let (x5, y5) = pen.point(qp, qs);
        canvas.set(x5, y5, head);
        label(
            canvas,
            &pen,
            edge,
            Anchor {
                start: (pp, ps),
                mid: (pp.saturating_add(qp)) / 2,
                straight: true,
            },
        );
        return;
    }
    jog(canvas, &pen, edge, [(pp, ps), (qp, qs)], head);
}

fn jog(canvas: &mut Canvas, pen: &Pen, edge: &Edge, ends: [(usize, usize); 2], head: char) {
    let [(pp, ps), (qp, qs)] = ends;
    let mid = pp.saturating_add(qp) / 2;
    if let Some((low, high)) = span(pp, mid) {
        pen.primary(canvas, ps, low, high);
    }
    let (toward, back) = pen.sideways(ps, qs);
    let (x6, y6) = pen.point(mid, ps);
    canvas.corner(x6, y6, pen.back() | toward);
    if let Some((low, high)) = between(ps, qs) {
        pen.secondary(canvas, mid, low, high);
    }
    let (x7, y7) = pen.point(mid, qs);
    canvas.corner(x7, y7, back | pen.ahead());
    if let Some((low, high)) = between(mid, qp) {
        pen.primary(canvas, qs, low, high);
    }
    let (x8, y8) = pen.point(qp, qs);
    canvas.set(x8, y8, head);
    label(
        canvas,
        pen,
        edge,
        Anchor {
            start: (pp, ps),
            mid,
            straight: false,
        },
    );
}

#[derive(Clone, Copy)]
struct Anchor {
    start: (usize, usize),
    mid: usize,
    straight: bool,
}

fn label(canvas: &mut Canvas, pen: &Pen, edge: &Edge, anchor: Anchor) {
    if edge.label.is_empty() {
        return;
    }
    let (pp, ps) = anchor.start;
    let (x, y) = if pen.dir.vertical() {
        pen.point(
            if anchor.straight { anchor.mid } else { pp },
            ps.saturating_add(2),
        )
    } else {
        pen.point(pp.saturating_add(1), ps.saturating_sub(1))
    };
    canvas.text(x, y, &edge.label);
}
