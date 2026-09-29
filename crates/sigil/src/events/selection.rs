use std::cell::RefCell;
use std::collections::HashMap;
use web_time::Instant;

use crate::events::handlers::EventHandlers;
use crate::events::{ClickConfig, ModifiersState, MouseButton, PointerEvent, PointerPhase};
use crate::geometry::Point;
use crate::hal::{HitPosition, TextAffinity};
use crate::kernel::{Cx, Node, NodeId};
use crate::layout::LayoutEngine;

use super::selection_walk::{
    endpoint_at, node_text, selected_ranges, selected_text, snap_focus, word_bounds,
};

pub struct SelectableText;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Endpoint {
    pub(crate) node: NodeId,
    pub(crate) byte: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct TextSelection {
    pub(crate) region: NodeId,
    pub(crate) anchor: Endpoint,
    pub(crate) focus: Endpoint,
    dragging: bool,
    moved: bool,
    pub(crate) word: Option<(usize, usize)>,
}

thread_local! {
    static STATE: RefCell<Option<TextSelection>> = const { RefCell::new(None) };
    static LAST_DOWN: RefCell<Option<(Instant, Point)>> = const { RefCell::new(None) };
}

pub(crate) fn clear() {
    STATE.with(|state| *state.borrow_mut() = None);
}

fn marked(node: &Node) -> bool {
    node.get_extension::<SelectableText>().is_some()
}

pub(crate) fn down(cx: &mut Cx, path: &[NodeId], event: &PointerEvent) {
    begin(cx, path, event);
}

pub(crate) fn drag(cx: &mut Cx, layout: &LayoutEngine, event: &mut PointerEvent) {
    if matches!(event.phase, PointerPhase::Move | PointerPhase::Up) {
        extend(cx, layout, event);
    }
}

pub(crate) fn end_drag() {
    STATE.with(|state| {
        if let Some(selection) = state.borrow_mut().as_mut() {
            selection.dragging = false;
        }
    });
}

pub(crate) fn escape(cx: &mut Cx) -> bool {
    if STATE.with(|state| state.borrow().is_some()) {
        clear();
        cx.mark_mutated();
        return true;
    }
    false
}

fn pick_region(cx: &Cx, path: &[NodeId]) -> Option<NodeId> {
    let pos = path
        .iter()
        .rposition(|id| cx.get(*id).is_some_and(marked))?;
    let interactive = path.get(pos + 1..).unwrap_or_default().iter().any(|id| {
        cx.get(*id)
            .and_then(|node| node.get_extension::<EventHandlers>())
            .is_some_and(EventHandlers::owns_pointer_drag)
    });
    if interactive {
        return None;
    }
    path.get(pos).copied()
}

fn begin(cx: &mut Cx, path: &[NodeId], event: &PointerEvent) {
    if event.button != Some(MouseButton::Left) {
        dismiss(cx);
        return;
    }
    let shift_only = event.modifiers == ModifiersState::SHIFT;
    if !event.modifiers.is_empty() && !shift_only {
        dismiss(cx);
        return;
    }
    let Some(region) = pick_region(cx, path) else {
        dismiss(cx);
        return;
    };
    let Some(endpoint) = endpoint_at(cx, region, event.position) else {
        dismiss(cx);
        return;
    };
    if shift_only {
        shift_extend(cx, region, endpoint);
        return;
    }
    let word = note_down(event.position)
        .then(|| node_text(cx, endpoint.node).and_then(|text| word_bounds(&text, endpoint.byte)))
        .flatten();
    if STATE.with(|state| state.borrow().is_some()) {
        cx.mark_mutated();
    }
    let (anchor, focus) = match word {
        Some((w0, w1)) => (
            Endpoint {
                node: endpoint.node,
                byte: w0,
            },
            Endpoint {
                node: endpoint.node,
                byte: w1,
            },
        ),
        None => (endpoint, endpoint),
    };
    STATE.with(|state| {
        *state.borrow_mut() = Some(TextSelection {
            region,
            anchor,
            focus,
            dragging: true,
            moved: word.is_some(),
            word,
        });
    });
}

fn note_down(point: Point) -> bool {
    let now = Instant::now();
    let double = LAST_DOWN.with(|state| {
        state.borrow().is_some_and(|(at, prev)| {
            ClickConfig::current().is_repeat(now.duration_since(at), prev, point)
        })
    });
    LAST_DOWN.with(|state| *state.borrow_mut() = Some((now, point)));
    double
}

fn shift_extend(cx: &mut Cx, region: NodeId, endpoint: Endpoint) {
    let existing = STATE
        .with(|state| *state.borrow())
        .filter(|sel| sel.region == region && live(cx, *sel));
    match existing {
        Some(sel) => {
            let (anchor, focus) = snap_focus(cx, sel, endpoint);
            STATE.with(|state| {
                *state.borrow_mut() = Some(TextSelection {
                    anchor,
                    focus,
                    dragging: true,
                    moved: true,
                    ..sel
                });
            });
            cx.mark_mutated();
        }
        None => STATE.with(|state| {
            *state.borrow_mut() = Some(TextSelection {
                region,
                anchor: endpoint,
                focus: endpoint,
                dragging: true,
                moved: false,
                word: None,
            });
        }),
    }
}

fn extend(cx: &mut Cx, layout: &LayoutEngine, event: &mut PointerEvent) {
    let Some(mut selection) =
        STATE.with(|state| state.borrow().as_ref().filter(|sel| sel.dragging).copied())
    else {
        return;
    };
    if let Some(endpoint) = endpoint_at(cx, selection.region, event.position) {
        let (anchor, focus) = snap_focus(cx, selection, endpoint);
        if focus != selection.focus || anchor != selection.anchor {
            selection.anchor = anchor;
            selection.focus = focus;
            selection.moved = true;
            crate::events::focus::reveal_focus(cx, layout, focus.node);
            cx.mark_mutated();
        }
    }
    if event.phase == PointerPhase::Move {
        STATE.with(|state| *state.borrow_mut() = Some(selection));
        return;
    }
    selection.dragging = false;
    if !selection.moved {
        clear();
        cx.mark_mutated();
        return;
    }
    event.stop_propagation();
    STATE.with(|state| *state.borrow_mut() = Some(selection));
    if cx.host_mode() == crate::kernel::HostMode::Cell {
        let text = selected_text(cx, selection);
        if !text.is_empty() {
            let _ = crate::clipboard::copy(&text);
        }
    }
}

fn dismiss(cx: &mut Cx) {
    if STATE.with(|state| state.borrow().is_some()) {
        clear();
        cx.mark_mutated();
    }
}

fn live(cx: &Cx, sel: TextSelection) -> bool {
    [sel.anchor.node, sel.focus.node]
        .iter()
        .all(|id| cx.get(*id).is_some())
}

pub(crate) fn copy_selection(cx: &Cx) -> bool {
    let Some(sel) = STATE.with(|state| state.borrow().as_ref().filter(|sel| sel.moved).copied())
    else {
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
    let _ = crate::clipboard::copy(&text);
    true
}

pub(crate) fn ranges(cx: &Cx) -> HashMap<NodeId, (usize, usize)> {
    let Some(sel) = STATE.with(|state| *state.borrow()) else {
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
