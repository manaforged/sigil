//! Event router implementation.

use accesskit::TreeUpdate;
use sigil_hal::{
    FileDropEvent, Point, PointerEvent, PointerPhase, ScrollEvent, ShellEvent, Shortcut,
};
use sigil_kernel::{Cx, NodeId};
use sigil_layout::LayoutEngine;
use smallvec::SmallVec;

use crate::EventHandlers;
use crate::focus::{FocusIntent, FocusManager};
use crate::hit_test::{HitTestCache, NodePath};
use crate::pointer_dispatch::{
    PointerTargets, dispatch_pointer_path, dispatch_pointer_target, dispatch_synthetic_target,
    pointer_target_for_node, pointer_targets, synthetic_leave_source,
};

mod keyboard;

/// Routes shell events through a composed node tree.
///
/// Pointer hit-test reads `layout` as it stands. The app frame
/// loop drains events before this frame's compute, so those boxes
/// are last frame's.
#[derive(Debug, Default)]
pub struct EventRouter {
    focus: FocusManager,
    /// Node the active IME composition belongs to, set on the focused
    /// node when a session opens and cleared on `Disabled` or when the
    /// owner stops being a valid dispatch target. `pub(crate)` — driven
    /// entirely by the `ime_handlers` sibling module, a non-descendant
    /// of `router` under Rust's module privacy rules.
    pub(crate) ime_session: crate::ime_handlers::ImeSession,
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
    ///
    /// Pointer targets come from `layout`'s published boxes. The app
    /// frame loop passes last frame's layout here.
    pub fn route(&mut self, cx: &mut Cx, layout: &LayoutEngine, event: ShellEvent) {
        self.focus.sync_scopes(cx, Some(layout));
        match event {
            ShellEvent::Pointer(event) => self.route_pointer(cx, layout, event),
            ShellEvent::Key(event) => self.route_key(cx, layout, event),
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
            ShellEvent::Scroll(event) => self.route_scroll(cx, layout, event),
            ShellEvent::Ime(event) => self.route_ime(cx, layout, &event),
            ShellEvent::FileDrop(event) => Self::route_file_drop(cx, &event),
            ShellEvent::Window(_) => {}
        }
    }

    /// Broadcast a file drag-and-drop to every node carrying a file-drop
    /// handler. The OS reports no drop position, so there is no target to
    /// hit-test: a drop is delivered to all registered handlers (typically
    /// one drop zone) which decide whether the file is theirs.
    fn route_file_drop(cx: &Cx, event: &FileDropEvent) {
        let mut stack: Vec<NodeId> = cx.roots().to_vec();
        while let Some(id) = stack.pop() {
            let Some(node) = cx.get(id) else {
                continue;
            };
            if let Some(handlers) = node.get_extension::<EventHandlers>() {
                handlers.invoke_file_drop(event);
            }
            stack.extend(node.children().iter().copied());
        }
    }

    /// Route a scroll (mouse wheel / trackpad) event by hit-testing
    /// `event.position` and bubbling the resulting path target-first.
    /// Hit-testing fresh (rather than reusing the cached hover path)
    /// lets a scroll arrive before any pointer-move event has built up
    /// hover state, which can happen on first touchpad use after a
    /// window opens. Handlers can call `event.stop_propagation()` to
    /// consume the event.
    fn route_scroll(&mut self, cx: &Cx, layout: &LayoutEngine, mut event: ScrollEvent) {
        let Some(path) = self.hit_cache.hit_path(cx, layout, event.position) else {
            return;
        };
        for id in path.iter().rev().copied() {
            let Some(node) = cx.get(id) else {
                continue;
            };
            let Some(handlers) = node.get_extension::<EventHandlers>() else {
                continue;
            };
            handlers.invoke_scroll(&mut event);
            if event.propagation_stopped() {
                return;
            }
        }
    }

    /// Return the currently focused node.
    #[must_use]
    pub const fn focus(&self) -> Option<NodeId> {
        self.focus.focus()
    }

    /// Set focus explicitly. Programmatic focus is treated as focus-visible
    /// (ring shown), matching keyboard focus.
    pub fn set_focus(&mut self, cx: &mut Cx, target: Option<NodeId>) {
        crate::note_focus_source(true);
        self.focus.set_focus(cx, target);
    }

    /// Advance focus through the tab ring (keyboard → focus-visible).
    /// Programmatic call — no layout in scope, so hidden-subtree ring
    /// filtering only applies on the routed-event path.
    pub fn focus_next(&mut self, cx: &mut Cx) {
        crate::note_focus_source(true);
        self.focus.focus_next(cx, None);
    }

    /// Move focus backward through the tab ring (keyboard → focus-visible).
    /// Programmatic call — see `focus_next` on the layout-less ring.
    pub fn focus_prev(&mut self, cx: &mut Cx) {
        crate::note_focus_source(true);
        self.focus.focus_prev(cx, None);
    }

    /// Keep focus on a live, laid-out target inside the active scope.
    ///
    /// Hosts call this after layout so reactive subtree replacement and
    /// newly-opened modal scopes cannot leave focus on stale or hidden nodes.
    pub fn focus_if_needed(&mut self, cx: &mut Cx, layout: &LayoutEngine) {
        self.focus.sync_scopes(cx, Some(layout));
        if self
            .focus
            .focus()
            .is_some_and(|id| self.focus.is_focusable_target(cx, layout, id))
        {
            return;
        }
        crate::note_focus_source(true);
        self.focus.focus_next(cx, Some(layout));
    }

    /// Move focus directionally (dpad / analog-stick shaped navigation),
    /// independent of Tab order. Resolves the nearest focusable node in
    /// `intent`'s direction from laid-out rect centers, falling back to
    /// Tab-order next/prev when nothing lies strictly in that direction
    /// so focus never dead-ends. The host maps hardware to `intent`;
    /// Sigil stays device-agnostic.
    pub fn focus_intent(&mut self, cx: &mut Cx, layout: &LayoutEngine, intent: FocusIntent) {
        crate::note_focus_source(true);
        self.focus.focus_intent(cx, layout, intent);
    }

    /// Build an AccessKit focus-only update.
    #[must_use]
    pub fn a11y_focus_update(&self) -> TreeUpdate {
        self.focus.a11y_focus_update()
    }

    /// Whether `target` is a legitimate focus target this frame —
    /// focusable, laid out, and inside the active focus scope. AT
    /// `Action::Focus` requests must pass this before `set_focus`.
    #[must_use]
    pub fn is_focus_target(&mut self, cx: &Cx, layout: &LayoutEngine, target: NodeId) -> bool {
        self.focus.is_focusable_target(cx, layout, target)
    }

    /// Whether `target` is a legitimate AT click target this frame —
    /// laid out, hit-test-enabled, and inside the active focus scope.
    #[must_use]
    pub fn is_click_target(&self, cx: &Cx, layout: &LayoutEngine, target: NodeId) -> bool {
        self.focus.is_hittable_target(cx, layout, target)
    }

    /// Test-only: current focus-scope stack depth.
    #[cfg(test)]
    #[must_use]
    pub const fn scope_depth(&self) -> usize {
        self.focus.scope_depth()
    }

    /// Whether `id` sits inside the topmost active focus scope. Used by
    /// IME dispatch gating (`ime_handlers.rs`) — see
    /// `FocusManager::is_within_active_scope`.
    #[must_use]
    pub(crate) fn ime_target_in_scope(&self, cx: &Cx, id: NodeId) -> bool {
        self.focus.is_within_active_scope(cx, id)
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
        // Focus the nearest focusable node on the hit path, walking leaf→root
        // (the path is root→leaf) so the innermost focusable wins. Checking
        // only the leaf misses composite widgets whose focusable outer node
        // wraps non-focusable children: a `text_field`'s click lands on its
        // inner text/caret node, so a leaf-only check never focuses the field
        // and keyboard input never routes to it.
        let target = path
            .iter()
            .rev()
            .copied()
            .find(|id| cx.get(*id).is_some_and(|node| node.a11y.focusable));
        if target.is_some() {
            // Pointer-initiated focus is NOT focus-visible: a mouse click
            // focuses the widget (so keyboard input routes there) but must
            // not raise the keyboard focus ring (`:focus-visible` semantics).
            crate::note_focus_source(false);
            self.focus.set_focus(cx, target);
        }
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

/// Lazily built index of nodes that advertise a semantic shortcut.
///
/// Rebuilt whenever `Cx::mutation_generation()` advances. Dispatch is
/// a linear scan over this compact list instead of a full tree walk.
/// Inline capacity of 8 keeps steady-state dispatch allocation-free
/// for typical menus; larger shortcut sets spill once per rebuild.
///
/// Field type only — the `Default` impl and `refresh`/`dispatch`
/// methods live in `router/keyboard.rs` alongside the rest of the
/// keyboard-routing concern; the struct itself stays here since it is
/// the type of `EventRouter::shortcuts`.
#[derive(Debug)]
struct ShortcutIndex {
    generation: u64,
    entries: SmallVec<[(Shortcut, NodeId); 8]>,
}
