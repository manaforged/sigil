use crate::hal::{Key, KeyEvent, KeyPhase, LogicalKey, ModifiersState, Shortcut};
use crate::kernel::{Cx, NodeId, Role};
use crate::layout::LayoutEngine;
use smallvec::SmallVec;

use super::{EventRouter, ShortcutIndex};
use crate::events::focus::FocusManager;
use crate::events::handlers::dispatch;
use crate::events::hit_test::visible;

impl EventRouter {
    pub(crate) fn route_key(&mut self, cx: &mut Cx, layout: &LayoutEngine, mut event: KeyEvent) {
        if event.phase != KeyPhase::Down {
            return;
        }
        self.shortcuts.refresh(cx);
        if self
            .shortcuts
            .dispatch(cx, &mut event, self.focused_accepts_text(cx))
            || event.propagation_stopped()
        {
            return;
        }
        crate::events::note_focus_source(true);
        if route_tab_key(&mut self.focus, cx, layout, &event) {
            return;
        }
        self.focus.focus_active_scope(cx, layout);
        self.dispatch_focused_key(cx, &mut event);
    }

    fn dispatch_focused_key(&mut self, cx: &mut Cx, event: &mut KeyEvent) {
        let Some(focused) = self.focus.focus().filter(|id| visible(cx, *id)) else {
            self.focus.set_focus(cx, None);
            return;
        };
        let path = path_to_node(cx, focused);
        dispatch_key_path(cx, &path, event);
    }

    fn focused_accepts_text(&self, cx: &Cx) -> bool {
        self.focus
            .focus()
            .filter(|id| visible(cx, *id))
            .and_then(|id| cx.get(id))
            .is_some_and(|node| matches!(node.a11y.role, Role::TextInput | Role::PasswordInput))
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
    dispatch(cx, id, |handlers| handlers.invoke_key(event, capture)).unwrap_or(false)
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

    fn dispatch(&self, cx: &Cx, event: &mut KeyEvent, text_focused: bool) -> bool {
        for (shortcut, id) in &self.entries {
            if !visible(cx, *id) {
                continue;
            }
            if text_focused && is_plain_char(shortcut) {
                continue;
            }
            if shortcut.matches(event) && invoke_key(cx, *id, event, false) {
                return true;
            }
        }
        false
    }
}

const fn is_plain_char(shortcut: &Shortcut) -> bool {
    matches!(shortcut.key, LogicalKey::Character(_))
        && shortcut
            .modifiers
            .intersection(
                ModifiersState::SHIFT
                    .union(ModifiersState::CONTROL)
                    .union(ModifiersState::ALT)
                    .union(ModifiersState::META),
            )
            .is_empty()
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
        return false;
    } else {
        focus.focus_next(cx, Some(layout));
    }
    true
}

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
