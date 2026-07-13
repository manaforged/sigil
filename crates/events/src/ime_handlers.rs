//! IME handler storage attached to a `kernel::Node` extension, and the
//! `EventRouter` methods that dispatch through it.
//!
//! Separate from [`crate::EventHandlers`] (pointer/key/scroll/focus)
//! because composition delivery is a different concern from spatial-input
//! dispatch — DR-051 §Decision-3 and the rust-hygiene god-struct rule both
//! name the same split. `EventRouter`'s own file (`router.rs`) sits at the
//! file-length hard cap, so the `route_ime` / `ime_caret_rect` /
//! `focus_wants_ime` methods live in this sibling `impl EventRouter` block
//! instead — legal in Rust, and all three only need the already-public
//! [`EventRouter::focus`] accessor, so no extra field visibility is needed.

use hal::{ImeEvent, Point, Rect};
use kernel::{Cx, NodeId};
use layout::{CaretGeometry, LayoutEngine};

use crate::EventRouter;

type ImeHandler = Box<dyn Fn(&ImeEvent) + 'static>;
type ImeEligible = Box<dyn Fn() -> bool>;

/// IME composition handlers attached to a node through the extension bag.
#[derive(Default)]
pub(crate) struct ImeHandlers {
    handlers: Vec<ImeHandler>,
    /// Live "am I currently eligible for IME composition" query — lets a
    /// widget's own disabled/read-only state gate dispatch independent
    /// of whether `.on_ime(...)` was ever registered. `None` (never set
    /// via `.ime_eligible(...)`) means always-eligible.
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

/// Node-relative caret anchor an IME-target widget publishes so the router
/// can resolve [`EventRouter::ime_caret_rect`] at end-of-Layout. Shared via
/// `Rc<Cell<_>>` (mirrors the interior-mutable `Rc<RefCell<_>>` shape
/// widgets already use for buffer state) so IME/commit handler closures —
/// which receive no `Cx` — can publish a fresh anchor without a node
/// mutation.
#[derive(Clone, Debug)]
pub(crate) struct CaretAnchorExt(pub(crate) std::rc::Rc<std::cell::Cell<Option<CaretGeometry>>>);

/// Caret-anchor rects have no natural width (a caret is a vertical bar);
/// this is the on-screen sliver width passed to the OS candidate-window
/// placement.
const CARET_ANCHOR_W: f32 = 2.0;

/// IME composition-session state (W5-IME HIGH-1 + the reviewer final-gate HIGH).
///
/// `Cancelled` is deliberately distinct from `Idle`: a session cancelled
/// by a focus/scope change has an OS composition still streaming events;
/// those quarantined events must not open a fresh session on the new
/// focus. Only the OS lifecycle boundary (`Disabled`, then `Enabled`)
/// leaves quarantine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ImeSession {
    /// No composition anywhere.
    #[default]
    Idle,
    /// Composition pinned to this node.
    Active(NodeId),
    /// Session cancelled mid-composition; residual events quarantined.
    Cancelled,
}

impl EventRouter {
    /// Dispatch one IME event to the node that owns the active
    /// composition session, not merely to whoever is focused right now.
    ///
    /// A session opens on `Enabled`/`Preedit` against the current focus
    /// (if it's a valid target) and stays pinned to that node — a focus
    /// or scope change away from the owner cancels the session (a
    /// synthetic `Disabled` reaches the old owner so its preedit clears)
    /// instead of letting a later `Commit` land on whatever is focused
    /// next. W5-IME HIGH-1.
    pub(crate) fn route_ime(&mut self, cx: &Cx, layout: &LayoutEngine, event: &ImeEvent) {
        if let ImeSession::Active(owner) = self.ime_session
            && (self.focus() != Some(owner) || !valid_ime_target(self, cx, layout, owner))
        {
            dispatch_to_owner(cx, owner, &ImeEvent::Disabled);
            // NOT Idle: the OS composition that owned this session is
            // still streaming. Collapsing to Idle would let this very
            // event re-adopt the new focus as owner (the reviewer final-gate
            // HIGH) — quarantine instead until the OS closes the old
            // session (Disabled) and opens a new one (Enabled).
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
            // Quarantine: the cancelled composition's own follow-up
            // events must never reach the newly focused node. Disabled
            // is the OS closing that old session — return to Idle.
            (ImeSession::Cancelled, ImeEvent::Disabled) => {
                self.ime_session = ImeSession::Idle;
                return;
            }
            // Quarantined residuals, and — separately — bare Commit/
            // Disabled with no session (adopting a fresh owner there
            // would re-deliver the exact misrouted event HIGH-1 flags):
            // both drop.
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

    /// Whether the focused node accepts IME composition: has registered
    /// an `on_ime` handler AND its own `.ime_eligible(...)` query (if
    /// any) currently returns `true`. Read once per frame by the app
    /// loop to drive `Shell::set_ime_allowed`.
    #[must_use]
    pub fn focus_wants_ime(&self, cx: &Cx) -> bool {
        self.focus()
            .and_then(|id| cx.get(id))
            .and_then(|node| node.get_extension::<ImeHandlers>())
            .is_some_and(ImeHandlers::is_eligible)
    }

    /// Resolve the focused IME target's caret rect in root (absolute)
    /// logical-pixel coordinates: the widget-published node-relative caret
    /// anchor combined with the node's laid-out origin. `None` when no IME
    /// target is focused, it hasn't published an anchor this frame, or the
    /// node has no computed layout yet.
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

/// Minimal, self-contained "is this a legitimate IME dispatch target"
/// check: present in the tree, laid out this frame, hit-testable,
/// inside the topmost active focus scope, and eligible per its own
/// `.ime_eligible(...)` query. Does not depend on the (unmerged)
/// AccessKit branch's fuller `is_focusable_target`/`is_within_active_scope`
/// validators — converge onto those once that branch lands.
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
