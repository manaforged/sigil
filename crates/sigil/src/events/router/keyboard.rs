use crate::events::{Key, KeyEvent, KeyPhase, LogicalKey, ModifiersState, Shortcut};
use crate::kernel::{Cx, NodeId, Role};
use crate::layout::LayoutEngine;
use smallvec::SmallVec;

use super::{EventRouter, ShortcutIndex};
use crate::events::focus::FocusManager;
use crate::events::handlers::{EventHandlers, dispatch};
use crate::events::hit_test::visible;
use crate::events::pointer_dispatch::path_to_node;

impl EventRouter {
    pub(crate) fn route_key(&mut self, cx: &mut Cx, layout: &LayoutEngine, mut event: KeyEvent) {
        if event.phase != KeyPhase::Down {
            return;
        }
        let focused = self.focused_kind(cx);
        if focused != Focused::Text
            && {
                let mods = event.normalized_modifiers();
                mods.meta() || (mods.ctrl() && focused != Focused::Terminal)
            }
            && matches!(&event.key, LogicalKey::Character(text) if text.eq_ignore_ascii_case("c"))
            && crate::events::selection::copy_selection(cx)
        {
            event.stop_propagation();
            return;
        }
        self.shortcuts.refresh(cx);
        if self.shortcuts.dispatch(cx, &mut event, focused) || event.propagation_stopped() {
            return;
        }
        if matches!(event.key, LogicalKey::Named(Key::Escape))
            && event.modifiers.is_empty()
            && crate::events::selection::escape(cx)
        {
            return;
        }
        crate::events::note_focus_source(true);
        if event.is_tab_navigation() && focused != Focused::Terminal {
            self.route_tab(cx, layout, &mut event);
            return;
        }
        self.focus.focus_active_scope(cx, layout);
        self.dispatch_focused_key(cx, &mut event);
    }

    fn dispatch_focused_key(&mut self, cx: &mut Cx, event: &mut KeyEvent) {
        let terminal_first =
            self.focused_kind(cx) == Focused::Terminal && !event.normalized_modifiers().meta();
        let Some(focused) = self.focus.focus().filter(|id| visible(cx, *id)) else {
            self.focus.set_focus(cx, None);
            return;
        };
        let path = path_to_node(cx, focused);
        if terminal_first {
            if !run_phase(cx, path.iter().rev().copied(), event, false)
                && !self.shortcuts.dispatch(cx, event, Focused::Other)
            {
                run_phase(cx, path.iter().copied(), event, true);
            }
        } else if !run_phase(cx, path.iter().copied(), event, true) {
            run_phase(cx, path.iter().rev().copied(), event, false);
        }
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

    fn focused_kind(&self, cx: &Cx) -> Focused {
        let role = self
            .focus
            .focus()
            .filter(|id| visible(cx, *id))
            .and_then(|id| cx.get(id))
            .map(|node| node.a11y.role);
        match role {
            Some(Role::TextInput | Role::PasswordInput) => Focused::Text,
            Some(Role::Terminal) => Focused::Terminal,
            _ => Focused::Other,
        }
    }
}

/// What kind of node has keyboard focus, which decides the shortcuts it leaves to the window.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Focused {
    /// A text field: plain characters type instead of firing shortcuts.
    Text,
    /// A terminal: every key goes to it except shortcuts held with the command key.
    Terminal,
    /// Anything else.
    Other,
}

fn run_phase(
    cx: &Cx,
    path: impl Iterator<Item = NodeId>,
    event: &mut KeyEvent,
    capture: bool,
) -> bool {
    for id in path {
        let _ = crate::events::handlers::fire(
            cx,
            id,
            crate::events::handlers::EventKind::Key,
            crate::events::handlers::Phase::of(capture),
            crate::events::handlers::EventArg::Key(event),
        );
        if event.propagation_stopped() {
            return true;
        }
    }
    false
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

    fn dispatch(&self, cx: &Cx, event: &mut KeyEvent, focused: Focused) -> bool {
        for (shortcut, id) in &self.entries {
            let yields = match focused {
                Focused::Text => shortcut.is_plain_char(),
                Focused::Terminal => !shortcut.modifiers.contains(ModifiersState::META),
                Focused::Other => false,
            };
            if !visible(cx, *id) || yields {
                continue;
            }
            if shortcut.matches(event)
                && dispatch(cx, *id, |handlers| {
                    handlers.dispatch(
                        crate::events::handlers::EventKind::Shortcut,
                        crate::events::handlers::Phase::Bubble,
                        &mut crate::events::handlers::EventArg::Key(event),
                    ) || handlers.dispatch(
                        crate::events::handlers::EventKind::Key,
                        crate::events::handlers::Phase::Bubble,
                        &mut crate::events::handlers::EventArg::Key(event),
                    )
                }) == Some(true)
            {
                return true;
            }
        }
        false
    }
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
    if event.modifiers.shift() {
        focus.focus_prev(cx, Some(layout));
    } else if focus
        .focus()
        .and_then(|id| cx.get(id))
        .is_some_and(|node| node.a11y.traps_tab)
    {
        return false;
    } else {
        focus.focus_next(cx, Some(layout));
    }
    if let Some(id) = focus.focus() {
        crate::events::focus::reveal_focus(cx, layout, id);
    }
    true
}
