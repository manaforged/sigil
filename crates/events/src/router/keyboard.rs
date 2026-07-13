//! Keyboard event routing: semantic shortcut dispatch, Tab-order
//! navigation, and focused-node key bubbling/capturing.
//!
//! Split out of `router.rs` — that file grew back over the 400-line
//! hygiene cap; this is the keyboard-routing concern (distinct from the
//! pointer/scroll dispatch that stays in the parent module) split into
//! its own child module, matching the `focus/scope.rs` and
//! `focus/targets.rs` precedent already in this crate.

use hal::{Key, KeyEvent, KeyPhase, LogicalKey, Shortcut};
use kernel::{Cx, NodeId};
use layout::LayoutEngine;
use smallvec::SmallVec;

use super::{EventRouter, ShortcutIndex};
use crate::EventHandlers;
use crate::focus::FocusManager;

impl EventRouter {
    pub(crate) fn route_key(&mut self, cx: &mut Cx, layout: &LayoutEngine, mut event: KeyEvent) {
        if event.phase != KeyPhase::Down {
            return;
        }
        self.shortcuts.refresh(cx);
        if self.shortcuts.dispatch(cx, &mut event) || event.propagation_stopped() {
            return;
        }
        // Tab moves focus via the keyboard → focus-visible (ring shown).
        crate::note_focus_source(true);
        if route_tab_key(&mut self.focus, cx, layout, &event) {
            return;
        }
        self.dispatch_focused_key(cx, &mut event);
    }

    fn dispatch_focused_key(&mut self, cx: &mut Cx, event: &mut KeyEvent) {
        let Some(focused) = self.focus.focus().filter(|id| cx.get(*id).is_some()) else {
            self.focus.set_focus(cx, None);
            return;
        };
        let path = path_to_node(cx, focused);
        dispatch_key_path(cx, &path, event);
    }
}

fn dispatch_key_path(cx: &Cx, path: &[NodeId], event: &mut KeyEvent) {
    for id in path {
        invoke_key(cx, *id, event, true);
        if event.propagation_stopped() {
            return;
        }
    }
    for id in path.iter().rev().copied() {
        invoke_key(cx, id, event, false);
        if event.propagation_stopped() {
            return;
        }
    }
}

fn invoke_key(cx: &Cx, id: NodeId, event: &mut KeyEvent, capture: bool) -> bool {
    cx.get(id)
        .and_then(|node| node.get_extension::<EventHandlers>())
        .is_some_and(|handlers| handlers.invoke_key(event, capture))
}

impl Default for ShortcutIndex {
    fn default() -> Self {
        Self {
            generation: u64::MAX,
            entries: SmallVec::new(),
        }
    }
}

impl ShortcutIndex {
    fn refresh(&mut self, cx: &Cx) {
        let generation = cx.mutation_generation();
        if self.generation == generation {
            return;
        }
        self.entries.clear();
        for root in cx.roots().iter().copied() {
            collect_shortcuts(cx, root, &mut self.entries);
        }
        self.generation = generation;
    }

    fn dispatch(&self, cx: &Cx, event: &mut KeyEvent) -> bool {
        for (shortcut, id) in &self.entries {
            if shortcut.matches(event) && invoke_key(cx, *id, event, false) {
                return true;
            }
        }
        false
    }
}

fn collect_shortcuts(cx: &Cx, id: NodeId, entries: &mut SmallVec<[(Shortcut, NodeId); 8]>) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if let Some(shortcut) = node.a11y.keyboard_shortcut.as_ref() {
        entries.push((shortcut.clone(), id));
    }
    for child in node.children().iter().copied() {
        collect_shortcuts(cx, child, entries);
    }
}

fn route_tab_key(
    focus: &mut FocusManager,
    cx: &mut Cx,
    layout: &LayoutEngine,
    event: &KeyEvent,
) -> bool {
    if event.key != LogicalKey::Named(Key::Tab) || event.modifiers.ctrl() || event.modifiers.alt() {
        return false;
    }
    if event.modifiers.shift() {
        focus.focus_prev(cx, Some(layout));
    } else if traps_tab(focus, cx) {
        // The focused node wants the raw Tab (code editor indent). Let the key fall
        // through to the focused-key dispatch instead of moving focus. Shift+Tab is
        // unaffected above, so there is always a keyboard way out.
        return false;
    } else {
        focus.focus_next(cx, Some(layout));
    }
    true
}

/// Whether the currently focused node opts to receive a plain `Tab` itself.
fn traps_tab(focus: &FocusManager, cx: &Cx) -> bool {
    focus
        .focus()
        .and_then(|id| cx.get(id))
        .is_some_and(|node| node.a11y.traps_tab)
}

fn path_to_node(cx: &Cx, target: NodeId) -> SmallVec<[NodeId; 64]> {
    let mut reversed = SmallVec::<[NodeId; 64]>::new();
    let mut current = Some(target);
    while let Some(id) = current {
        reversed.push(id);
        current = cx.parent(id);
    }
    reversed.reverse();
    reversed
}
