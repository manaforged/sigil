use crate::hal::{
    FileDropEvent, Point, PointerEvent, PointerPhase, ScrollEvent, ShellEvent, Shortcut,
};
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutEngine;
use accesskit::TreeUpdate;
use smallvec::SmallVec;

use crate::events::EventHandlers;
use crate::events::focus::{FocusIntent, FocusManager};
use crate::events::hit_test::{HitTestCache, NodePath};
use crate::events::pointer_dispatch::{
    PointerTargets, dispatch_pointer_path, dispatch_pointer_target, dispatch_synthetic_target,
    pointer_target_for_node, pointer_targets, synthetic_leave_source,
};

mod keyboard;

#[derive(Debug, Default)]
pub struct EventRouter {
    focus: FocusManager,
    pub(crate) ime_session: crate::events::ime_handlers::ImeSession,
    pointer_capture: Option<NodeId>,
    hover_targets: PointerTargets,
    last_pointer: Option<PointerEvent>,
    shortcuts: ShortcutIndex,
    hit_cache: HitTestCache,
}

impl EventRouter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn route(&mut self, cx: &mut Cx, layout: &LayoutEngine, event: ShellEvent) {
        self.focus.sync_scopes(cx, Some(layout));
        self.flush_focus_requests(cx, layout);
        match event {
            ShellEvent::Pointer(event) => self.route_pointer(cx, layout, event),
            ShellEvent::Key(event) => self.route_key(cx, layout, event),
            ShellEvent::Window(crate::hal::WindowEvent::Focused(false)) => {
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

    #[must_use]
    pub const fn focus(&self) -> Option<NodeId> {
        self.focus.focus()
    }

    pub fn set_focus(&mut self, cx: &mut Cx, target: Option<NodeId>) {
        crate::events::note_focus_source(true);
        self.focus.set_focus(cx, target);
    }

    pub fn flush_focus_requests(&mut self, cx: &mut Cx, layout: &LayoutEngine) {
        let pending = crate::events::focus::take_focus_requests();
        if pending.is_empty() {
            return;
        }
        crate::events::note_focus_source(true);
        for id in pending {
            if self.focus.is_focusable_target(cx, layout, id) {
                self.focus.set_focus(cx, Some(id));
            }
        }
    }

    pub fn focus_next(&mut self, cx: &mut Cx) {
        crate::events::note_focus_source(true);
        self.focus.focus_next(cx, None);
    }

    pub fn focus_prev(&mut self, cx: &mut Cx) {
        crate::events::note_focus_source(true);
        self.focus.focus_prev(cx, None);
    }

    pub fn focus_if_needed(&mut self, cx: &mut Cx, layout: &LayoutEngine) {
        self.focus.sync_scopes(cx, Some(layout));
        if self
            .focus
            .focus()
            .is_some_and(|id| self.focus.is_focusable_target(cx, layout, id))
        {
            return;
        }
        crate::events::note_focus_source(true);
        self.focus.focus_next(cx, Some(layout));
    }

    pub fn focus_intent(&mut self, cx: &mut Cx, layout: &LayoutEngine, intent: FocusIntent) {
        crate::events::note_focus_source(true);
        self.focus.focus_intent(cx, layout, intent);
    }

    #[must_use]
    pub fn a11y_focus_update(&self) -> TreeUpdate {
        self.focus.a11y_focus_update()
    }

    #[must_use]
    pub fn is_focus_target(&mut self, cx: &Cx, layout: &LayoutEngine, target: NodeId) -> bool {
        self.focus.is_focusable_target(cx, layout, target)
    }

    #[must_use]
    pub fn is_click_target(&self, cx: &Cx, layout: &LayoutEngine, target: NodeId) -> bool {
        self.focus.is_hittable_target(cx, layout, target)
    }

    #[cfg(test)]
    #[must_use]
    pub const fn scope_depth(&self) -> usize {
        self.focus.scope_depth()
    }

    #[must_use]
    pub(crate) fn ime_target_in_scope(&self, cx: &Cx, id: NodeId) -> bool {
        self.focus.is_within_active_scope(cx, id)
    }

    fn route_pointer(&mut self, cx: &mut Cx, layout: &LayoutEngine, mut event: PointerEvent) {
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
        let capture_requester =
            dispatch_pointer_path(cx, &next_targets, &mut event, global_position);
        if event.capture_requested() {
            self.pointer_capture = capture_requester.or_else(|| path.last().copied());
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
            .iter()
            .rev()
            .copied()
            .find(|id| cx.get(*id).is_some_and(|node| node.a11y.is_focusable()));
        if target.is_some() {
            crate::events::note_focus_source(false);
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

#[derive(Debug)]
struct ShortcutIndex {
    generation: u64,
    entries: SmallVec<[(Shortcut, NodeId); 8]>,
}
