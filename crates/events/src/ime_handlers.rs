use sigil_hal::{ImeEvent, Point, Rect};
use sigil_kernel::{Cx, NodeId};
use sigil_layout::{CaretGeometry, LayoutEngine};

use crate::EventRouter;

type ImeHandler = Box<dyn Fn(&ImeEvent) + 'static>;
type ImeEligible = Box<dyn Fn() -> bool>;

#[derive(Default)]
pub(crate) struct ImeHandlers {
    handlers: Vec<ImeHandler>,
    eligible: Option<ImeEligible>,
}

impl ImeHandlers {
    pub(crate) fn add(&mut self, handler: ImeHandler) {
        self.handlers.push(handler);
    }

    pub(crate) fn set_eligible(&mut self, f: ImeEligible) {
        self.eligible = Some(f);
    }

    pub(crate) fn is_eligible(&self) -> bool {
        self.eligible.as_ref().is_none_or(|f| f())
    }

    pub(crate) fn invoke(&self, event: &ImeEvent) {
        for handler in &self.handlers {
            handler(event);
        }
    }
}

impl core::fmt::Debug for ImeHandlers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ImeHandlers").finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CaretAnchorExt(pub(crate) std::rc::Rc<std::cell::Cell<Option<CaretGeometry>>>);

const CARET_ANCHOR_W: f32 = 2.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ImeSession {
    #[default]
    Idle,
    Active(NodeId),
    Cancelled,
}

impl EventRouter {
    pub(crate) fn route_ime(&mut self, cx: &Cx, layout: &LayoutEngine, event: &ImeEvent) {
        if let ImeSession::Active(owner) = self.ime_session
            && (self.focus() != Some(owner) || !valid_ime_target(self, cx, layout, owner))
        {
            dispatch_to_owner(cx, owner, &ImeEvent::Disabled);
            self.ime_session = ImeSession::Cancelled;
        }
        match (self.ime_session, event) {
            (ImeSession::Idle, ImeEvent::Enabled | ImeEvent::Preedit(..))
            | (ImeSession::Cancelled, ImeEvent::Enabled) => {
                self.ime_session = self
                    .focus()
                    .filter(|id| valid_ime_target(self, cx, layout, *id))
                    .map_or(ImeSession::Idle, ImeSession::Active);
            }
            (ImeSession::Cancelled, ImeEvent::Disabled) => {
                self.ime_session = ImeSession::Idle;
                return;
            }
            (ImeSession::Cancelled, ImeEvent::Preedit(..) | ImeEvent::Commit(_))
            | (ImeSession::Idle, ImeEvent::Commit(_) | ImeEvent::Disabled) => return,
            (ImeSession::Active(_), _) => {}
        }
        let ImeSession::Active(owner) = self.ime_session else {
            return;
        };
        dispatch_to_owner(cx, owner, event);
        if matches!(event, ImeEvent::Disabled) {
            self.ime_session = ImeSession::Idle;
        }
    }

    #[must_use]
    pub fn focus_wants_ime(&self, cx: &Cx) -> bool {
        self.focus()
            .and_then(|id| cx.get(id))
            .and_then(|node| node.get_extension::<ImeHandlers>())
            .is_some_and(ImeHandlers::is_eligible)
    }

    #[must_use]
    pub fn ime_caret_rect(&self, cx: &Cx, layout: &LayoutEngine) -> Option<Rect> {
        let focused = self.focus()?;
        let node = cx.get(focused)?;
        let local = node.get_extension::<CaretAnchorExt>()?.0.get()?;
        let origin = layout.result(focused)?.rect().origin;
        Some(Rect::new(
            Point::new(origin.x + local.x, origin.y + local.top),
            CARET_ANCHOR_W,
            local.height,
        ))
    }
}

fn dispatch_to_owner(cx: &Cx, id: NodeId, event: &ImeEvent) {
    if let Some(handlers) = cx
        .get(id)
        .and_then(|node| node.get_extension::<ImeHandlers>())
    {
        handlers.invoke(event);
    }
}

fn valid_ime_target(router: &EventRouter, cx: &Cx, layout: &LayoutEngine, id: NodeId) -> bool {
    let Some(node) = cx.get(id) else {
        return false;
    };
    node.a11y.hittable
        && layout.result(id).is_some()
        && router.ime_target_in_scope(cx, id)
        && node
            .get_extension::<ImeHandlers>()
            .is_some_and(ImeHandlers::is_eligible)
}
