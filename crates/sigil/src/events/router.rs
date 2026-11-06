use crate::events::{Event, PointerEvent, PointerPhase, ScrollEvent, Shortcut};
use crate::geometry::Point;
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutEngine;
#[cfg(any(test, feature = "kittest"))]
use accesskit::TreeUpdate;
use smallvec::SmallVec;

use crate::events::focus::{FocusIntent, FocusManager};
use crate::events::hit_test::{HitTestCache, NodePath, visible};
use crate::events::pointer_dispatch::{
    PointerTargets, dispatch_pointer_path, dispatch_pointer_target, dispatch_synthetic_target,
    local_point, pointer_target_for_node, pointer_targets, synthetic_leave_source,
};

mod file_drop;
mod gesture;
mod keyboard;
mod paste;

pub(crate) use gesture::GestureHandlers;
pub(crate) use paste::PasteHandlers;

kittest_pub! {
    #[derive(Debug, Default)]
    pub struct EventRouter {
        pub(crate) focus: FocusManager,
        pointer_capture: Option<NodeId>,
        hover_targets: PointerTargets,
        last_pointer: Option<PointerEvent>,
        shortcuts: ShortcutIndex,
        hit_cache: HitTestCache,
        hover_seen: Option<(u64, u64, (u32, u32))>,
        link_press: Option<std::sync::Arc<str>>,
    }
}

impl EventRouter {
    kittest_pub! {
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }
    }

    pub(crate) const fn set_visual_transforms(&mut self, enabled: bool) {
        self.hit_cache.visual_transforms = enabled;
    }

    kittest_pub! {
        pub fn route(&mut self, cx: &mut Cx, layout: &LayoutEngine, event: Event) {
            self.focus.sync_scopes(cx, Some(layout));
            match event {
                Event::Pointer(event) => self.route_pointer(cx, layout, event),
                Event::Key(event) => self.route_key(cx, layout, event),
                Event::Window(crate::events::WindowEvent::Focused(false)) => {
                    self.focus_lost(cx, layout);
                }
                Event::Scroll(event) => self.route_scroll(cx, layout, event),
                Event::Gesture(event) => self.route_gesture(cx, layout, event),
                Event::Ime(event) => self.route_ime(cx, layout, &event),
                Event::FileDrop(event) => self.route_file_drop(cx, layout, &event),
                Event::Paste(text) => self.route_paste(cx, layout, &text),
                Event::Window(_) => {}
            }
        }
    }

    fn route_scroll(&mut self, cx: &Cx, layout: &LayoutEngine, mut event: ScrollEvent) {
        self.bubble(cx, layout, event.position, |id, local| {
            event.position = local;
            let _ = crate::events::handlers::fire(
                cx,
                id,
                crate::events::handlers::EventKind::Scroll,
                crate::events::handlers::Phase::Bubble,
                crate::events::handlers::EventArg::Scroll(&mut event),
            );
            event.propagation_stopped()
        });
    }

    fn bubble(
        &mut self,
        cx: &Cx,
        layout: &LayoutEngine,
        global: Point,
        mut visit: impl FnMut(NodeId, Point) -> bool,
    ) {
        let Some(path) = self.hit_cache.hit_path(cx, layout, global) else {
            return;
        };
        let targets = pointer_targets(cx, layout, &path, self.hit_cache.uses_transforms());
        for target in targets.iter().rev().copied() {
            if visit(target.id, local_point(cx, target, global)) {
                return;
            }
        }
    }

    kittest_pub! {
        #[must_use]
        pub const fn focus(&self) -> Option<NodeId> {
            self.focus.focus()
        }
    }

    kittest_pub! {
        pub fn set_focus(&mut self, cx: &mut Cx, target: Option<NodeId>) {
            crate::events::note_focus_source(true);
            self.focus.set_focus(cx, target);
        }
    }

    kittest_pub! {
        pub fn flush_focus_requests(&mut self, cx: &mut Cx, layout: &LayoutEngine) {
            let pending = crate::events::focus::take_focus_requests();
            if pending.is_empty() {
                return;
            }
            crate::events::note_focus_source(true);
            self.focus.sync_scopes(cx, Some(layout));
            for id in pending {
                if self.focus.is_focusable_target(cx, layout, id) {
                    self.focus.set_focus(cx, Some(id));
                }
            }
            if let Some(id) = self.focus.focus() {
                crate::events::focus::reveal_focus(cx, layout, id);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn tab_edges(&self, cx: &Cx) -> crate::events::focus::TabEdges {
        self.focus.tab_edges(cx)
    }

    #[cfg(any(test, feature = "kittest"))]
    pub fn focus_next(&mut self, cx: &mut Cx, layout: &LayoutEngine) {
        crate::events::note_focus_source(true);
        self.focus.focus_next(cx, Some(layout));
    }

    #[cfg(any(test, feature = "kittest"))]
    pub fn focus_prev(&mut self, cx: &mut Cx, layout: &LayoutEngine) {
        crate::events::note_focus_source(true);
        self.focus.focus_prev(cx, Some(layout));
    }

    kittest_pub! {
        pub fn focus_if_needed(&mut self, cx: &mut Cx, layout: &LayoutEngine) {
            self.focus.sync_scopes(cx, Some(layout));
            if self.focus.holds_live_focus(cx) {
                return;
            }
            crate::events::note_focus_source(crate::style::cell_size().is_some());
            self.focus.focus_first(cx, layout);
        }
    }

    kittest_pub! {
        pub fn focus_intent(&mut self, cx: &mut Cx, layout: &LayoutEngine, intent: FocusIntent) {
            crate::events::note_focus_source(true);
            self.focus.focus_intent(cx, layout, intent);
        }
    }

    #[must_use]
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    pub fn a11y_focus_update(&self) -> TreeUpdate {
        self.focus.a11y_focus_update()
    }

    #[cfg(any(test, feature = "kittest"))]
    #[cfg(not(feature = "kittest"))]
    pub(crate) fn a11y_focus_update(&self) -> TreeUpdate {
        self.focus.a11y_focus_update()
    }

    kittest_pub! {
        #[must_use]
        pub fn is_focus_target(&mut self, cx: &Cx, layout: &LayoutEngine, target: NodeId) -> bool {
            self.focus.is_focusable_target(cx, layout, target)
        }
    }

    kittest_pub! {
        #[must_use]
        pub fn is_click_target(&self, cx: &Cx, layout: &LayoutEngine, target: NodeId) -> bool {
            self.focus.is_hittable_target(cx, layout, target)
        }
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
        if event.phase != PointerPhase::Down {
            crate::events::selection::drag(cx, layout, &mut event);
        }
        if self.route_captured_pointer(cx, layout, &mut event, global_position) {
            return;
        }
        let path = self.hit_cache.hit_path(cx, layout, event.position);
        if event.phase == PointerPhase::Down
            && !path.as_ref().is_some_and(|path| in_popup(cx, path))
            && crate::widgets::popup_close_light()
        {
            self.end_press(event.phase);
            return;
        }
        let Some(path) = path else {
            self.clear_hover(cx, &event);
            self.end_press(event.phase);
            return;
        };
        if event.phase == PointerPhase::Down {
            crate::events::selection::down(cx, &path, &event);
        }
        let next_targets = pointer_targets(cx, layout, &path, self.hit_cache.uses_transforms());
        self.update_hover(cx, &next_targets, &event);
        self.focus_pointer_target(cx, &path, event.phase);
        let capture_requester =
            dispatch_pointer_path(cx, &next_targets, &mut event, global_position);
        self.follow_link(cx, &path, &event, global_position);
        if event.capture_requested() {
            self.pointer_capture = capture_requester.or_else(|| path.last().copied());
        }
        self.end_press(event.phase);
    }

    fn follow_link(&mut self, cx: &Cx, path: &NodePath, event: &PointerEvent, at: Point) {
        match event.phase {
            PointerPhase::Down => {
                self.link_press = crate::events::links::link_at(cx, path, at);
            }
            PointerPhase::Up => {
                let pressed = self.link_press.take();
                let released = crate::events::links::link_at(cx, path, at);
                if let (Some(href), false) = (
                    released.filter(|href| pressed.as_ref() == Some(href)),
                    event.propagation_stopped(),
                ) {
                    crate::events::links::activate(cx, path, &href);
                }
            }
            _ => {}
        }
    }

    fn route_captured_pointer(
        &mut self,
        cx: &Cx,
        layout: &LayoutEngine,
        event: &mut PointerEvent,
        global_position: Point,
    ) -> bool {
        let Some(captured) = self.pointer_capture.filter(|id| visible(cx, *id)) else {
            self.pointer_capture = None;
            return false;
        };
        if !matches!(
            event.phase,
            PointerPhase::Move | PointerPhase::Up | PointerPhase::Cancel
        ) {
            return false;
        }
        event.set_captured(true);
        dispatch_pointer_target(
            cx,
            pointer_target_for_node(cx, layout, captured, self.hit_cache.visual_transforms),
            event,
            global_position,
        );
        self.end_press(event.phase);
        true
    }

    pub(crate) fn invoke_node(&self, cx: &Cx, layout: &LayoutEngine, target: NodeId) {
        let hovered = self.hover_targets.iter().any(|hover| hover.id == target);
        crate::events::pointer_dispatch::invoke_node(cx, layout, target, hovered);
    }

    pub(crate) fn reconcile_hover(&mut self, cx: &Cx, layout: &LayoutEngine) {
        if self.pointer_capture.is_some() {
            return;
        }
        let Some(pointer) = self.last_pointer.clone() else {
            return;
        };
        let seen = (
            cx.mutation_generation(),
            layout.generation(),
            (pointer.position.x.to_bits(), pointer.position.y.to_bits()),
        );
        if self.hover_seen.replace(seen) == Some(seen) {
            return;
        }
        let Some(path) = self.hit_cache.hit_path(cx, layout, pointer.position) else {
            self.clear_hover(cx, &pointer);
            return;
        };
        let next_targets = pointer_targets(cx, layout, &path, self.hit_cache.uses_transforms());
        self.update_hover(cx, &next_targets, &pointer);
    }

    fn update_hover(&mut self, cx: &Cx, next: &PointerTargets, source: &PointerEvent) {
        let common = self
            .hover_targets
            .iter()
            .zip(next.iter())
            .take_while(|(a, b)| a.id == b.id)
            .count();
        if common == next.len() && common == self.hover_targets.len() {
            return;
        }
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

    fn focus_lost(&mut self, cx: &Cx, layout: &LayoutEngine) {
        let source = self
            .last_pointer
            .clone()
            .unwrap_or_else(synthetic_leave_source);
        if let Some(captured) = self.pointer_capture.take().filter(|id| visible(cx, *id)) {
            let target =
                pointer_target_for_node(cx, layout, captured, self.hit_cache.visual_transforms);
            dispatch_synthetic_target(cx, target, &source, PointerPhase::Cancel);
        }
        self.link_press = None;
        crate::events::selection::end_drag();
        self.clear_hover(cx, &source);
    }

    fn clear_hover(&mut self, cx: &Cx, source: &PointerEvent) {
        for target in self.hover_targets.iter().rev().copied() {
            dispatch_synthetic_target(cx, target, source, PointerPhase::Leave);
        }
        self.hover_targets.clear();
    }

    fn end_press(&mut self, phase: PointerPhase) {
        if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) {
            self.pointer_capture = None;
            self.link_press = None;
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

#[derive(Debug)]
struct ShortcutIndex {
    generation: u64,
    entries: SmallVec<[(Shortcut, NodeId); 8]>,
}

fn in_popup(cx: &Cx, path: &NodePath) -> bool {
    path.iter().any(|id| {
        cx.get(*id)
            .is_some_and(|node| node.a11y.focus_scope == Some(crate::kernel::FocusScopeKind::Popup))
    })
}
