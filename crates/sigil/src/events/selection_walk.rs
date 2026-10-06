use std::collections::HashMap;
use std::rc::Rc;

use unicode_segmentation::UnicodeSegmentation;

use crate::geometry::{Point, Rect};
use crate::kernel::{Cx, Node, NodeId, NodeType};
use crate::layout::{LayoutResult, with_text_geometry};
use crate::style::{StyleBinding, TextStyle, Theme, text_style_for};

use super::selection::{Endpoint, TextSelection, Unit};

pub(super) fn hidden(node: &Node) -> bool {
    node.get_extension::<StyleBinding>()
        .is_some_and(StyleBinding::hides)
}

struct Inherit<'a> {
    theme: &'a Theme,
    text_style: Rc<TextStyle>,
}

fn default_theme() -> &'static Theme {
    use std::sync::OnceLock;
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(Theme::default)
}

pub(super) fn endpoint_at(cx: &Cx, region: NodeId, point: Point) -> Option<Endpoint> {
    let node = cx.get(region)?;
    let theme: &Theme = node
        .get_extension::<Theme>()
        .map_or_else(|| default_theme(), |theme| theme);
    let inherit = Inherit {
        theme,
        text_style: Rc::new(TextStyle::default()),
    };
    let mut best: Option<(Gap, Endpoint)> = None;
    walk_text(cx, region, &inherit, point, &mut best);
    best.map(|(_, endpoint)| endpoint)
}

fn walk_text(
    cx: &Cx,
    id: NodeId,
    inherit: &Inherit<'_>,
    point: Point,
    best: &mut Option<(Gap, Endpoint)>,
) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if hidden(node) {
        return;
    }
    let theme = node.get_extension::<Theme>().unwrap_or(inherit.theme);
    let text_style = node
        .get_extension::<StyleBinding>()
        .and_then(|binding| TextStyle::layered_over_binding(&inherit.text_style, binding))
        .map_or_else(|| Rc::clone(&inherit.text_style), Rc::new);
    if let NodeType::Text { content, wrap, .. } = &node.kind
        && let Some(rect) = node.get_extension::<LayoutResult>().map(LayoutResult::rect)
    {
        let distance = gap(rect, point);
        if best.is_none_or(|(score, _)| distance < score) {
            let text = content.resolve();
            let style = text_style_for(node, theme, &text_style);
            let local = Point::new(point.x - rect.origin.x, point.y - rect.origin.y);
            let width = wrap.then_some(rect.width);
            let byte =
                with_text_geometry(|engine| engine.hit(&text, &style, width, local).byte_index)
                    .unwrap_or(0);
            *best = Some((distance, Endpoint { node: id, byte }));
        }
    }
    let next = Inherit { theme, text_style };
    for child in node.children().iter().copied() {
        walk_text(cx, child, &next, point, best);
    }
}

type Gap = (f32, f32);

fn gap(rect: Rect, point: Point) -> Gap {
    let bottom = rect.origin.y + rect.height;
    let right = rect.origin.x + rect.width;
    let vertical = if point.y < rect.origin.y {
        rect.origin.y - point.y
    } else if point.y > bottom {
        point.y - bottom
    } else {
        0.0
    };
    let horizontal = if point.x < rect.origin.x {
        rect.origin.x - point.x
    } else if point.x > right {
        point.x - right
    } else {
        0.0
    };
    (vertical, horizontal)
}

pub(super) fn node_text(cx: &Cx, id: NodeId) -> Option<String> {
    let node = cx.get(id)?;
    match &node.kind {
        NodeType::Text { content, .. } => Some(content.resolve()),
        _ => None,
    }
}

pub(super) fn word_bounds(text: &str, byte: usize) -> Option<(usize, usize)> {
    for (start, word) in text.unicode_word_indices() {
        let end = start + word.len();
        if byte >= start && byte < end {
            return Some((start, end));
        }
    }
    None
}

const CELL_EDGE: f32 = 0.01;

pub(super) fn cell_bounds(cx: &Cx, region: NodeId, point: Point) -> Option<(usize, usize)> {
    if cx.host_mode() != crate::kernel::HostMode::Cell {
        return None;
    }
    let reach = crate::Host::CELL / 2.0 - CELL_EDGE;
    let lead = endpoint_at(cx, region, Point::new(point.x - reach, point.y))?;
    let tail = endpoint_at(cx, region, Point::new(point.x + reach, point.y))?;
    (lead.node == tail.node).then(|| (lead.byte.min(tail.byte), lead.byte.max(tail.byte)))
}

pub(super) fn snap_focus(
    cx: &Cx,
    sel: TextSelection,
    hit: Endpoint,
    point: Point,
) -> (Endpoint, Endpoint) {
    let Some((w0, w1)) = sel.word else {
        return (sel.anchor, hit);
    };
    let (order, _) = text_nodes(cx, sel.region);
    let anchor_at = order.get(&sel.anchor.node).copied().unwrap_or(0);
    let hit_at = order.get(&hit.node).copied().unwrap_or(usize::MAX);
    let forward = hit_at > anchor_at || (hit_at == anchor_at && hit.byte >= sel.anchor.byte);
    let span = match sel.unit {
        Unit::Word => node_text(cx, hit.node).and_then(|text| word_bounds(&text, hit.byte)),
        Unit::Cell => cell_bounds(cx, sel.region, point),
    };
    let word = span.unwrap_or((hit.byte, hit.byte));
    let focus_byte = if forward { word.1 } else { word.0 };
    let anchor_byte = if forward { w0 } else { w1 };
    (
        Endpoint {
            node: sel.anchor.node,
            byte: anchor_byte,
        },
        Endpoint {
            node: hit.node,
            byte: focus_byte,
        },
    )
}

fn selected_range(
    sel: TextSelection,
    order: &HashMap<NodeId, usize>,
    node: NodeId,
    len: usize,
) -> Option<(usize, usize)> {
    let at = *order.get(&node)?;
    let anchor = *order.get(&sel.anchor.node)?;
    let focus = *order.get(&sel.focus.node)?;
    let (lo, hi) = if (anchor, sel.anchor.byte) <= (focus, sel.focus.byte) {
        (sel.anchor, sel.focus)
    } else {
        (sel.focus, sel.anchor)
    };
    if at < anchor.min(focus) || at > anchor.max(focus) {
        return None;
    }
    let start = if at == anchor.min(focus) {
        lo.byte.min(len)
    } else {
        0
    };
    let end = if at == anchor.max(focus) {
        hi.byte.min(len)
    } else {
        len
    };
    (start < end).then_some((start, end))
}

pub(super) type TextNodes = (HashMap<NodeId, usize>, Vec<(NodeId, String)>);

fn collect_text(cx: &Cx, id: NodeId, texts: &mut Vec<(NodeId, String)>) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if hidden(node) {
        return;
    }
    if let NodeType::Text { content, .. } = &node.kind {
        texts.push((id, content.resolve()));
    }
    for child in node.children().iter().copied() {
        collect_text(cx, child, texts);
    }
}

pub(super) fn text_nodes(cx: &Cx, region: NodeId) -> TextNodes {
    let mut texts = Vec::new();
    collect_text(cx, region, &mut texts);
    let order = texts
        .iter()
        .enumerate()
        .map(|(index, (id, _))| (*id, index))
        .collect();
    (order, texts)
}

pub(super) fn selected_text(cx: &Cx, sel: TextSelection) -> String {
    let (order, texts) = text_nodes(cx, sel.region);
    texts
        .iter()
        .filter_map(|(id, text)| {
            let (start, end) = selected_range(sel, &order, *id, text.len())?;
            let start = floor_boundary(text, start);
            let end = ceil_boundary(text, end).max(start);
            Some(text[start..end].to_string())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn selected_ranges(cx: &Cx, sel: TextSelection) -> HashMap<NodeId, (usize, usize)> {
    let (order, texts) = text_nodes(cx, sel.region);
    texts
        .iter()
        .filter_map(|(id, text)| {
            selected_range(sel, &order, *id, text.len()).map(|range| (*id, range))
        })
        .collect()
}

fn floor_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}
