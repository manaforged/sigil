use std::cell::RefCell;
use std::collections::HashMap;

use crate::events::handlers::EventHandlers;
use crate::events::pointer_dispatch::Clicks;
use crate::events::{Modifiers, MouseButton, PointerEvent, PointerPhase};
use crate::geometry::Point;
use crate::hal::{HitPosition, TextAffinity};
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutEngine;

use super::selection_walk::{
    cell_bounds, endpoint_at, node_text, selected_ranges, selected_text, snap_focus, word_bounds,
};

pub(crate) struct SelectableText;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Endpoint {
    pub(crate) node: NodeId,
    pub(crate) byte: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unit {
    Word,
    Cell,
}

#[derive(Clone, Copy)]
pub(crate) struct TextSelection {
    host: u64,
    pub(crate) region: NodeId,
    pub(crate) anchor: Endpoint,
    pub(crate) focus: Endpoint,
    dragging: bool,
    moved: bool,
    pub(crate) word: Option<(usize, usize)>,
    pub(crate) unit: Unit,
}

thread_local! {
    static STATE: RefCell<Option<TextSelection>> = const { RefCell::new(None) };
    static CLICKS: RefCell<Clicks<()>> = RefCell::new(Clicks::default());
}

pub(crate) fn clear() {
    STATE.set(None);
}

pub(crate) fn drag(cx: &Cx, layout: &LayoutEngine, event: &mut PointerEvent) {
    match event.phase {
        PointerPhase::Move | PointerPhase::Up => extend(cx, layout, event),
        PointerPhase::Cancel => end_drag(),
        _ => {}
    }
}

pub(crate) fn end_drag() {
    STATE.with_borrow_mut(|state| {
        if let Some(selection) = state.as_mut() {
            selection.dragging = false;
        }
    });
}

pub(crate) fn drag_active() -> bool {
    STATE.with_borrow(|state| state.is_some_and(|sel| sel.dragging))
}

pub(crate) fn escape() -> bool {
    let had = STATE.take().is_some();
    if had {
        crate::reactive::Runtime::mark_frame_dirty();
    }
    had
}

fn pick_region(cx: &Cx, path: &[NodeId]) -> Option<NodeId> {
    let pos = path.iter().rposition(|id| {
        cx.get(*id)
            .is_some_and(|node| node.extension::<SelectableText>().is_some())
    })?;
    let interactive = path.get(pos + 1..).unwrap_or_default().iter().any(|id| {
        cx.get(*id).is_some_and(|node| {
            node.a11y.role == crate::kernel::Role::Button
                || node
                    .extension::<EventHandlers>()
                    .is_some_and(EventHandlers::owns_pointer_drag)
        })
    });
    if interactive {
        None
    } else {
        path.get(pos).copied()
    }
}

pub(crate) fn down(cx: &Cx, path: &[NodeId], event: &PointerEvent) {
    let shift_only = event.modifiers == Modifiers::SHIFT;
    let plain =
        event.button == Some(MouseButton::Left) && (event.modifiers.is_empty() || shift_only);
    let hit = plain
        .then(|| pick_region(cx, path))
        .flatten()
        .and_then(|region| Some((region, endpoint_at(cx, region, event.position)?)));
    let Some((region, endpoint)) = hit else {
        escape();
        return;
    };
    if shift_only {
        shift_extend(cx, region, endpoint, event.position);
        return;
    }
    if STATE.with_borrow(Option::is_some) {
        crate::reactive::Runtime::mark_frame_dirty();
    }
    let (span, unit) = press_span(cx, region, endpoint, event.position);
    let (anchor, focus) =
        span.filter(|_| unit == Unit::Word)
            .map_or((endpoint, endpoint), |(w0, w1)| {
                (
                    Endpoint {
                        byte: w0,
                        ..endpoint
                    },
                    Endpoint {
                        byte: w1,
                        ..endpoint
                    },
                )
            });
    STATE.set(Some(TextSelection {
        host: cx.instance_id(),
        region,
        anchor,
        focus,
        dragging: true,
        moved: unit == Unit::Word,
        word: span,
        unit,
    }));
}

fn press_span(
    cx: &Cx,
    region: NodeId,
    endpoint: Endpoint,
    point: Point,
) -> (Option<(usize, usize)>, Unit) {
    let word = note_down(point)
        .then(|| node_text(cx, endpoint.node).and_then(|text| word_bounds(&text, endpoint.byte)))
        .flatten();
    word.map_or_else(
        || (cell_bounds(cx, region, point), Unit::Cell),
        |span| (Some(span), Unit::Word),
    )
}

fn note_down(point: Point) -> bool {
    CLICKS.with_borrow_mut(|clicks| clicks.press((), point)) >= 2
}

fn shift_extend(cx: &Cx, region: NodeId, endpoint: Endpoint, point: Point) {
    let existing = current(cx).filter(|sel| sel.region == region && live(cx, *sel));
    let host = cx.instance_id();
    let next = existing.map_or(
        TextSelection {
            host,
            region,
            anchor: endpoint,
            focus: endpoint,
            dragging: true,
            moved: false,
            word: None,
            unit: Unit::Word,
        },
        |sel| {
            let (anchor, focus) = snap_focus(cx, sel, endpoint, point);
            TextSelection {
                anchor,
                focus,
                dragging: true,
                moved: true,
                ..sel
            }
        },
    );
    STATE.set(Some(next));
    if existing.is_some() {
        crate::reactive::Runtime::mark_frame_dirty();
    }
}

fn extend(cx: &Cx, layout: &LayoutEngine, event: &mut PointerEvent) {
    let Some(mut selection) = current(cx).filter(|sel| sel.dragging) else {
        return;
    };
    if let Some(endpoint) = endpoint_at(cx, selection.region, event.position) {
        let (anchor, focus) = snap_focus(cx, selection, endpoint, event.position);
        if focus != selection.focus || anchor != selection.anchor {
            selection.anchor = anchor;
            selection.focus = focus;
            selection.moved = true;
            crate::events::focus::reveal_focus(cx, layout, focus.node);
            crate::reactive::Runtime::mark_frame_dirty();
        }
    }
    if event.phase == PointerPhase::Move {
        STATE.set(Some(selection));
        return;
    }
    selection.dragging = false;
    if !selection.moved {
        clear();
        crate::reactive::Runtime::mark_frame_dirty();
        return;
    }
    event.stop_propagation();
    STATE.set(Some(selection));
    if cx.host_mode() == crate::kernel::HostMode::Cell {
        let text = selected_text(cx, selection);
        if !text.is_empty() {
            deliver(cx, selection.region, &text);
        }
    }
}

fn deliver(cx: &Cx, region: NodeId, text: &str) {
    let result = crate::clipboard::copy(text);
    let mut at = Some(region);
    while let Some(id) = at {
        if crate::events::handlers::fire(
            cx,
            id,
            crate::events::handlers::EventKind::SelectionCopy,
            crate::events::handlers::Phase::Bubble,
            crate::events::handlers::EventArg::SelectionCopy(text, &result),
        ) == Some(true)
        {
            return;
        }
        at = cx.parent(id);
    }
}

fn current(cx: &Cx) -> Option<TextSelection> {
    STATE.with_borrow(|state| state.filter(|sel| sel.host == cx.instance_id()))
}

fn live(cx: &Cx, sel: TextSelection) -> bool {
    [sel.anchor.node, sel.focus.node]
        .iter()
        .all(|id| cx.get(*id).is_some())
}

pub(crate) fn copy_selection(cx: &Cx) -> bool {
    let Some(sel) = current(cx).filter(|sel| sel.moved) else {
        return false;
    };
    if !live(cx, sel) {
        clear();
        return false;
    }
    let text = selected_text(cx, sel);
    if text.is_empty() {
        return false;
    }
    deliver(cx, sel.region, &text);
    true
}

pub(crate) fn ranges(cx: &Cx) -> HashMap<NodeId, (usize, usize)> {
    let Some(sel) = current(cx) else {
        return HashMap::new();
    };
    if !live(cx, sel) {
        clear();
        return HashMap::new();
    }
    selected_ranges(cx, sel)
}

pub(crate) const fn selection_hit(byte_index: usize) -> HitPosition {
    HitPosition {
        byte_index,
        affinity: TextAffinity::Downstream,
    }
}
