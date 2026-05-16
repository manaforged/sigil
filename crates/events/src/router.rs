//! Event router implementation.

use accesskit::TreeUpdate;
use sigil_hal::{
    Key, KeyEvent, KeyPhase, LogicalKey, Point, PointerEvent, PointerPhase, ShellEvent, Shortcut,
};
use sigil_kernel::{Cx, NodeId};
use sigil_layout::LayoutEngine;
use smallvec::SmallVec;

use crate::EventHandlers;
use crate::focus::FocusManager;
use crate::hit_test::{HitTestCache, NodePath};
use crate::pointer_dispatch::{
    PointerTargets, dispatch_pointer_path, dispatch_pointer_target, dispatch_synthetic_target,
    pointer_target_for_node, pointer_targets, synthetic_leave_source,
};

/// Routes shell events through a composed node tree.
#[derive(Debug, Default)]
pub struct EventRouter {
    focus: FocusManager,
    pointer_capture: Option<NodeId>,
    // Hover path cached with absolute origins so that Enter/Leave
    // synthesis on each transition is O(depth), not O(depth²). Rebuilding
    // origins per node via `path_to_node` per Enter/Leave was the 5x
    // regression caught on route_hover_transition_depth_20.
    hover_targets: PointerTargets,
    last_pointer: Option<PointerEvent>,
    shortcuts: ShortcutIndex,
    hit_cache: HitTestCache,
}

impl EventRouter {
    /// Construct an empty router.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Route one shell event through the tree and layout state.
    pub fn route(&mut self, cx: &mut Cx, layout: &LayoutEngine, event: ShellEvent) {
        match event {
            ShellEvent::Pointer(event) => self.route_pointer(cx, layout, event),
            ShellEvent::Key(event) => self.route_key(cx, event),
            ShellEvent::Window(sigil_hal::WindowEvent::Focused(false)) => {
                // Window lost focus: the pointer is effectively no longer
                // over anything. Dispatch Leave through every node in the
                // hover path so widgets (buttons, etc.) can clear their
                // hover/pressed state instead of freezing visually until
                // the window regains focus.
                self.pointer_capture = None;
                let source = self
                    .last_pointer
                    .clone()
                    .unwrap_or_else(synthetic_leave_source);
                self.clear_hover(cx, &source);
            }
            ShellEvent::Window(_) | ShellEvent::Ime(_) | ShellEvent::Scroll(_) => {}
        }
    }

    /// Return the currently focused node.
    #[must_use]
    pub const fn focus(&self) -> Option<NodeId> {
        self.focus.focus()
    }

    /// Set focus explicitly.
    pub fn set_focus(&mut self, cx: &mut Cx, target: Option<NodeId>) {
        self.focus.set_focus(cx, target);
    }

    /// Advance focus through the tab ring.
    pub fn focus_next(&mut self, cx: &mut Cx) {
        self.focus.focus_next(cx);
    }

    /// Move focus backward through the tab ring.
    pub fn focus_prev(&mut self, cx: &mut Cx) {
        self.focus.focus_prev(cx);
    }

    /// Build an AccessKit focus-only update.
    #[must_use]
    pub fn a11y_focus_update(&self) -> TreeUpdate {
        self.focus.a11y_focus_update()
    }

    fn route_pointer(&mut self, cx: &mut Cx, layout: &LayoutEngine, mut event: PointerEvent) {
        // Remember the last real pointer event so that synthesised Leaves
        // (for window blur, pointer capture loss, etc.) carry the actual
        // last-known position and modifier state instead of zeros.
        self.last_pointer = Some(event.clone());
        let global_position = event.position;
        if self.route_captured_pointer(cx, layout, &mut event, global_position) {
            return;
        }
        let Some(path) = self.hit_cache.hit_path(cx, layout, event.position) else {
            self.clear_hover(cx, &event);
            self.release_capture_on_terminal_phase(event.phase);
            return;
        };
        let next_targets = pointer_targets(layout, &path);
        self.update_hover(cx, &next_targets, &event);
        self.focus_pointer_target(cx, &path, event.phase);
        dispatch_pointer_path(cx, &next_targets, &mut event, global_position);
        if event.capture_requested() {
            self.pointer_capture = path.last().copied();
        }
        self.release_capture_on_terminal_phase(event.phase);
    }

    fn route_captured_pointer(
        &mut self,
        cx: &Cx,
        layout: &LayoutEngine,
        event: &mut PointerEvent,
        global_position: Point,
    ) -> bool {
        let Some(captured) = self.pointer_capture.filter(|id| cx.get(*id).is_some()) else {
            self.pointer_capture = None;
            return false;
        };
        if !matches!(event.phase, PointerPhase::Move | PointerPhase::Up) {
            return false;
        }
        event.set_captured(true);
        dispatch_pointer_target(
            cx,
            pointer_target_for_node(cx, layout, captured),
            event,
            global_position,
        );
        self.release_capture_on_terminal_phase(event.phase);
        true
    }

    // Diff the cached hover path (with origins) against the freshly
    // computed target list and fire synthetic Leave/Enter only on the
    // nodes whose membership changed.
    fn update_hover(&mut self, cx: &Cx, next: &PointerTargets, source: &PointerEvent) {
        if ids_match(&self.hover_targets, next) {
            return;
        }
        let common = target_common_prefix_len(&self.hover_targets, next);
        if let Some(leaving) = self.hover_targets.get(common..) {
            for target in leaving.iter().rev().copied() {
                dispatch_synthetic_target(cx, target, source, PointerPhase::Leave);
            }
        }
        if let Some(entering) = next.get(common..) {
            for target in entering.iter().copied() {
                dispatch_synthetic_target(cx, target, source, PointerPhase::Enter);
            }
        }
        self.hover_targets.clear();
        self.hover_targets.extend(next.iter().copied());
    }

    fn clear_hover(&mut self, cx: &Cx, source: &PointerEvent) {
        for target in self.hover_targets.iter().rev().copied() {
            dispatch_synthetic_target(cx, target, source, PointerPhase::Leave);
        }
        self.hover_targets.clear();
    }

    const fn release_capture_on_terminal_phase(&mut self, phase: PointerPhase) {
        if matches!(
            phase,
            PointerPhase::Up | PointerPhase::Leave | PointerPhase::Cancel
        ) {
            self.pointer_capture = None;
        }
    }

    fn focus_pointer_target(&mut self, cx: &mut Cx, path: &NodePath, phase: PointerPhase) {
        if phase != PointerPhase::Down {
            return;
        }
        let target = path
            .last()
            .copied()
            .filter(|id| cx.get(*id).is_some_and(|node| node.a11y.focusable));
        if target.is_some() {
            self.focus.set_focus(cx, target);
        }
    }

    fn route_key(&mut self, cx: &mut Cx, mut event: KeyEvent) {
        if event.phase != KeyPhase::Down {
            return;
        }
        self.shortcuts.refresh(cx);
        if self.shortcuts.dispatch(cx, &mut event) || event.propagation_stopped() {
            return;
        }
        if route_tab_key(&mut self.focus, cx, &event) {
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

fn ids_match(left: &PointerTargets, right: &PointerTargets) -> bool {
    left.len() == right.len() && left.iter().zip(right.iter()).all(|(a, b)| a.id == b.id)
}

fn target_common_prefix_len(left: &PointerTargets, right: &PointerTargets) -> usize {
    left.iter()
        .zip(right.iter())
        .take_while(|(a, b)| a.id == b.id)
        .count()
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

/// Lazily built index of nodes that advertise a semantic shortcut.
///
/// Rebuilt whenever `Cx::mutation_generation()` advances. Dispatch is
/// a linear scan over this compact list instead of a full tree walk.
/// Inline capacity of 8 keeps steady-state dispatch allocation-free
/// for typical menus; larger shortcut sets spill once per rebuild.
#[derive(Debug)]
struct ShortcutIndex {
    generation: u64,
    entries: SmallVec<[(Shortcut, NodeId); 8]>,
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
    for child in node.children.iter().copied() {
        collect_shortcuts(cx, child, entries);
    }
}

fn route_tab_key(focus: &mut FocusManager, cx: &mut Cx, event: &KeyEvent) -> bool {
    if event.key != LogicalKey::Named(Key::Tab) || event.modifiers.ctrl() || event.modifiers.alt() {
        return false;
    }
    if event.modifiers.shift() {
        focus.focus_prev(cx);
    } else {
        focus.focus_next(cx);
    }
    true
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
