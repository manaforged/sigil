use std::rc::Rc;
use std::sync::Arc;

use crate::events::hit_test::NodePath;
use crate::events::pointer_dispatch::path_to_node;
use crate::geometry::{Point, Rect};
use crate::hal::{HitPosition, TextAffinity};
use crate::kernel::{Cx, Node, NodeId, NodeType};
use crate::layout::{LayoutResult, with_text_geometry};
use crate::style::{StyleBinding, TextRuns, TextStyle, Theme, text_style_for};

pub(crate) struct LinkText {
    pub(crate) text: String,
    pub(crate) style: TextStyle,
    pub(crate) rect: Rect,
    pub(crate) width: Option<f32>,
    pub(crate) runs: TextRuns,
}

pub(crate) fn has_links(node: &Node) -> bool {
    matches!(node.kind, NodeType::Text { .. })
        && node
            .extension::<TextRuns>()
            .is_some_and(|runs| runs.0.iter().any(|run| run.face.link.is_some()))
}

pub(crate) fn link_text(cx: &Cx, id: NodeId) -> Option<LinkText> {
    let node = cx.get(id).filter(|node| has_links(node))?;
    let NodeType::Text { content, wrap, .. } = &node.kind else {
        return None;
    };
    let runs = node.extension::<TextRuns>()?.clone();
    let rect = node.extension::<LayoutResult>().map(LayoutResult::rect)?;
    let mut theme = crate::style::theme::fallback_theme();
    let mut base = Rc::new(TextStyle::default());
    for step in path_to_node(cx, id) {
        let Some(at) = cx.get(step) else {
            continue;
        };
        theme = at.extension::<Theme>().unwrap_or(theme);
        if let Some(layered) = at
            .extension::<StyleBinding>()
            .and_then(|binding| TextStyle::layered_over_binding(&base, binding))
        {
            base = Rc::new(layered);
        }
    }
    Some(LinkText {
        text: content.resolve(),
        style: text_style_for(node, theme, &base),
        rect,
        width: wrap.then_some(rect.width),
        runs,
    })
}

pub(crate) fn link_ranges(text: &LinkText) -> Vec<(usize, usize, Arc<str>)> {
    let mut out: Vec<(usize, usize, Arc<str>)> = Vec::new();
    for run in text.runs.0.iter() {
        let Some(href) = run.face.link.clone() else {
            continue;
        };
        match out.last_mut() {
            Some(last) if last.1 == run.start && last.2 == href => last.1 = run.end,
            _ => out.push((run.start, run.end, href)),
        }
    }
    out
}

pub(crate) fn range_rects(text: &LinkText, start: usize, end: usize) -> Vec<Rect> {
    let at = |byte_index| HitPosition {
        byte_index,
        affinity: TextAffinity::Downstream,
    };
    with_text_geometry(|geometry| {
        geometry.selection_rects(&text.text, &text.style, text.width, (at(start), at(end)))
    })
    .unwrap_or_default()
}

pub(crate) fn link_at(cx: &Cx, path: &NodePath, global: Point) -> Option<Arc<str>> {
    let text = path.iter().rev().find_map(|id| link_text(cx, *id))?;
    let byte = byte_at(&text, global)?;
    link_ranges(&text)
        .into_iter()
        .find(|(start, end, _)| (*start..*end).contains(&byte))
        .map(|(_, _, href)| href)
}

pub(crate) fn byte_at(text: &LinkText, global: Point) -> Option<usize> {
    let local = Point::new(global.x - text.rect.origin.x, global.y - text.rect.origin.y);
    with_text_geometry(|geometry| {
        geometry
            .hit(&text.text, &text.style, text.width, local)
            .byte_index
    })
}

pub(crate) fn activate(cx: &Cx, path: &NodePath, href: &str) {
    for id in path.iter().rev().copied() {
        if crate::events::handlers::fire(
            cx,
            id,
            crate::events::handlers::EventKind::Link,
            crate::events::handlers::Phase::Bubble,
            crate::events::handlers::EventArg::Link(href),
        ) == Some(true)
        {
            return;
        }
    }
}
