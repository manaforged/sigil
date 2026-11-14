use crate::events::ImeEvent;
use crate::events::handlers::{EventArg, EventHandlers, EventKind, Phase, fire};
#[cfg(window_host)]
use crate::geometry::{Point, Rect};
use crate::kernel::{Cx, Node, NodeId};
use crate::layout::CaretGeometry;
use crate::layout::LayoutEngine;

use crate::events::EventRouter;

pub(crate) struct ImeEligible(pub(crate) Box<dyn Fn() -> bool>);

impl core::fmt::Debug for ImeEligible {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ImeEligible").finish_non_exhaustive()
    }
}

fn wants_ime(node: &Node) -> bool {
    node.extension::<EventHandlers>()
        .is_some_and(|handlers| handlers.handles(EventKind::Ime))
        && node
            .extension::<ImeEligible>()
            .is_none_or(|eligible| (eligible.0)())
}

#[cfg_attr(
    not(window_host),
    allow(dead_code, reason = "only the window host reads the caret anchor")
)]
pub(crate) struct CaretAnchorExt(pub(crate) Box<dyn Fn() -> Option<CaretGeometry>>);

impl core::fmt::Debug for CaretAnchorExt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CaretAnchorExt").finish_non_exhaustive()
    }
}

#[cfg(window_host)]
const CARET_ANCHOR_W: f32 = 2.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ImeSession {
    #[default]
    Idle,
    Active(NodeId),
    Cancelled,
}

impl ImeSession {
    pub(crate) fn end(&mut self, cx: &Cx) {
        if let Self::Active(owner) = *self {
            dispatch_to_owner(cx, owner, &ImeEvent::Disabled);
            *self = Self::Cancelled;
        }
    }
}

impl EventRouter {
    pub(crate) fn route_ime(&mut self, cx: &Cx, layout: &LayoutEngine, event: &ImeEvent) {
        if let ImeSession::Active(owner) = self.focus.ime_session
            && (self.focus() != Some(owner) || !valid_ime_target(self, cx, layout, owner))
        {
            self.focus.ime_session.end(cx);
        }
        match (self.focus.ime_session, event) {
            (ImeSession::Idle, ImeEvent::Enabled | ImeEvent::Preedit(..))
            | (ImeSession::Cancelled, ImeEvent::Enabled) => {
                self.focus.ime_session = self
                    .focus()
                    .filter(|id| valid_ime_target(self, cx, layout, *id))
                    .map_or(ImeSession::Idle, ImeSession::Active);
            }
            (ImeSession::Cancelled, ImeEvent::Disabled) => {
                self.focus.ime_session = ImeSession::Idle;
                return;
            }
            (ImeSession::Cancelled, ImeEvent::Preedit(..) | ImeEvent::Commit(_))
            | (ImeSession::Idle, ImeEvent::Commit(_) | ImeEvent::Disabled) => return,
            (ImeSession::Active(_), _) => {}
        }
        let ImeSession::Active(owner) = self.focus.ime_session else {
            return;
        };
        dispatch_to_owner(cx, owner, event);
        if matches!(event, ImeEvent::Disabled) {
            self.focus.ime_session = ImeSession::Idle;
        }
    }

    kittest_pub! {
        #[cfg(window_host)]
        #[must_use]
        #[doc(hidden)]
        pub fn focus_wants_ime(&self, cx: &Cx) -> bool {
            self.focus()
                .and_then(|id| cx.get(id))
                .is_some_and(wants_ime)
        }
    }

    kittest_pub! {
        #[cfg(window_host)]
        #[must_use]
        #[doc(hidden)]
        pub fn ime_caret_rect(&self, cx: &Cx, layout: &LayoutEngine) -> Option<Rect> {
            let focused = self.focus()?;
            let node = cx.get(focused)?;
            let anchor = node.extension::<CaretAnchorExt>()?;
            let local = crate::reactive::untrack(|| (anchor.0)())?;
            let origin = layout.result(focused)?.rect().origin;
            Some(Rect::new(
                Point::new(origin.x + local.x, origin.y + local.top),
                CARET_ANCHOR_W,
                local.height,
            ))
        }
    }
}

fn dispatch_to_owner(cx: &Cx, id: NodeId, event: &ImeEvent) {
    let _ = fire(cx, id, EventKind::Ime, Phase::Bubble, EventArg::Ime(event));
}

fn valid_ime_target(router: &EventRouter, cx: &Cx, layout: &LayoutEngine, id: NodeId) -> bool {
    let Some(node) = cx.get(id) else {
        return false;
    };
    node.a11y.hittable
        && layout.result(id).is_some()
        && router.ime_target_in_scope(cx, id)
        && wants_ime(node)
}
