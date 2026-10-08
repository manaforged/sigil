use crate::events::{Key, KeyEvent, KeyPhase, LogicalKey, ModifiersState, Shortcut};
use crate::kernel::{Cx, NodeId, Role};
use crate::layout::LayoutEngine;
use smallvec::SmallVec;

use super::{EventRouter, ShortcutIndex};
use crate::events::focus::FocusManager;
use crate::events::handlers::{EventHandlers, dispatch};
use crate::events::hit_test::visible;

impl EventRouter {
    pub(crate) fn route_key(&mut self, cx: &mut Cx, layout: &LayoutEngine, mut event: KeyEvent) {
        if event.phase != KeyPhase::Down {
            return;
        }
        if !self.focused_accepts_text(cx)
            && {
                let mods = event.normalized_modifiers();
                mods.meta() || mods.ctrl()
            }
            && matches!(&event.key, LogicalKey::Character(text) if text.eq_ignore_ascii_case("c"))
            && crate::events::selection::copy_selection(cx)
        {
            event.stop_propagation();
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
        if matches!(event.key, LogicalKey::Named(Key::Escape))
            && event.modifiers.is_empty()
            && crate::events::selection::escape(cx)
        {
            return;
        }
        crate::events::note_focus_source(true);
        if event.is_tab_navigation() {
            self.route_tab(cx, layout, &mut event);
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

    fn route_tab(&mut self, cx: &mut Cx, layout: &LayoutEngine, event: &mut KeyEvent) {
        let path = self
            .focus
            .focus()
            .filter(|id| visible(cx, *id))
            .map(|focused| path_to_node(cx, focused))
            .unwrap_or_default();
        if run_phase(cx, path.iter().copied(), event, true) {
            return;
        }
        if route_tab_key(&mut self.focus, cx, layout, event) {
            return;
        }
        run_phase(cx, path.iter().rev().copied(), event, false);
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
    if run_phase(cx, path.iter().copied(), event, true) {
        return;
    }
    run_phase(cx, path.iter().rev().copied(), event, false);
}

fn run_phase(
    cx: &Cx,
    path: impl Iterator<Item = NodeId>,
    event: &mut KeyEvent,
    capture: bool,
) -> bool {
    for id in path {
        invoke_key(cx, id, event, capture);
        if event.propagation_stopped() {
            return true;
        }
    }
    false
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
            if text_focused && shortcut.is_plain_char() {
                continue;
            }
            if shortcut.matches(event) && invoke_shortcut(cx, *id, event) {
                return true;
            }
        }
        false
    }
}

fn invoke_shortcut(cx: &Cx, id: NodeId, event: &mut KeyEvent) -> bool {
    dispatch(cx, id, |handlers| {
        handlers.invoke_shortcut(event) || handlers.invoke_key(event, false)
    })
    .unwrap_or(false)
}

impl Shortcut {
    const fn is_plain_char(&self) -> bool {
        matches!(self.key, LogicalKey::Character(_))
            && self
                .modifiers
                .intersection(
                    ModifiersState::SHIFT
                        .union(ModifiersState::CONTROL)
                        .union(ModifiersState::ALT)
                        .union(ModifiersState::META),
                )
                .is_empty()
    }
}

fn collect_shortcuts(cx: &Cx, id: NodeId, entries: &mut SmallVec<[(Shortcut, NodeId); 8]>) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if let Some(shortcut) = node.a11y.keyboard_shortcut.as_ref() {
        entries.push((shortcut.clone(), id));
    }
    if let Some(handlers) = node.get_extension::<EventHandlers>() {
        entries.extend(handlers.shortcuts().map(|shortcut| (shortcut.clone(), id)));
    }
    for child in node.children().iter().copied() {
        collect_shortcuts(cx, child, entries);
    }
}

impl KeyEvent {
    fn is_tab_navigation(&self) -> bool {
        self.key == LogicalKey::Named(Key::Tab) && !self.modifiers.ctrl() && !self.modifiers.alt()
    }
}

fn route_tab_key(
    focus: &mut FocusManager,
    cx: &mut Cx,
    layout: &LayoutEngine,
    event: &KeyEvent,
) -> bool {
    if !event.is_tab_navigation() {
        return false;
    }
    if event.modifiers.shift() {
        focus.focus_prev(cx, Some(layout));
    } else if traps_tab(focus, cx) {
        return false;
    } else {
        focus.focus_next(cx, Some(layout));
    }
    if let Some(id) = focus.focus() {
        crate::events::focus::reveal_focus(cx, layout, id);
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
