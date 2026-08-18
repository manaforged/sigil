//! Focus-stack methods on [`EntityCx`].
//!
//! Extracted from `cx.rs` to keep that file under the 400-line cap.
//! Five methods that all touch the runtime's focus stack:
//! [`EntityCx::focus_handle`] (mint), [`EntityCx::focus`] /
//! [`EntityCx::blur`] (push/remove), [`EntityCx::focused`] /
//! [`EntityCx::painting`] (read).

use crate::cx::EntityCx;
use crate::focus::FocusHandle;
use crate::owner::on_cleanup;
use crate::runtime::with_rt;

impl<T: 'static> EntityCx<'_, T> {
    /// Mint a new [`FocusHandle`] owned by this context's entity.
    ///
    /// The handle is stable for the entity's lifetime. When the
    /// owning entity (or any ancestor owner) is disposed, the handle
    /// is automatically evicted from the focus stack via the
    /// owner-cascade `on_cleanup` hook (DR-046 §4).
    ///
    /// Caller binds the returned handle to a field for later
    /// [`focus`](Self::focus) / [`blur`](Self::blur) /
    /// [`action`](Self::action) calls.
    #[must_use]
    pub fn focus_handle(&mut self) -> FocusHandle {
        let handle = FocusHandle::new();
        let id = handle.id();
        // Owner-cascade cleanup: when the surrounding reactive owner
        // disposes, evict this handle from the focus stack. Idempotent
        // with an explicit `blur` — eviction just scans the stack.
        on_cleanup(move || {
            with_rt(|rt| rt.focus_stack.retain(|other| *other != id));
        });
        handle
    }

    /// Push `handle` onto the focus stack as the innermost focus.
    ///
    /// If `handle` is already on the stack it is moved to the
    /// innermost (deepest) position — this matches the intuitive
    /// behaviour of "click the inner panel, then click the outer
    /// panel, expect the outer panel to receive Tab/Escape next."
    pub fn focus(&mut self, handle: &FocusHandle) {
        let id = handle.id();
        with_rt(|rt| {
            crate::phase::assert_paint_safe(rt, "EntityCx::focus");
            rt.focus_stack.retain(|other| *other != id);
            rt.focus_stack.push(id);
        });
    }

    /// Remove `handle` from the focus stack. No-op if not present.
    pub fn blur(&mut self, handle: &FocusHandle) {
        let id = handle.id();
        with_rt(|rt| {
            crate::phase::assert_paint_safe(rt, "EntityCx::blur");
            rt.focus_stack.retain(|other| *other != id);
        });
    }

    /// Returns true if the reactive runtime is currently in
    /// [`crate::DrawPhase::Paint`].
    ///
    /// Widgets may branch on this to choose between an immediate-write
    /// path (allowed in Effect/Layout/Idle) and a deferred path during
    /// Paint. Reactive reads (`Signal::get`, `Entity::read`,
    /// `focused`) are always legal regardless of phase.
    #[must_use]
    pub fn painting(&self) -> bool {
        with_rt(|rt| rt.phase == crate::DrawPhase::Paint)
    }

    /// True if `handle` is currently somewhere on the focus stack.
    ///
    /// The stack semantically represents "every region that holds
    /// focus in the chain from root to leaf," so any presence counts —
    /// not just the innermost.
    #[must_use]
    pub fn focused(&self, handle: &FocusHandle) -> bool {
        let id = handle.id();
        with_rt(|rt| rt.focus_stack.contains(&id))
    }
}
