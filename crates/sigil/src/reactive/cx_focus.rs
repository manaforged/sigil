use crate::reactive::cx::EntityCx;
use crate::reactive::focus::FocusHandle;
use crate::reactive::owner::on_cleanup;
use crate::reactive::runtime::with_rt;

impl<T: 'static> EntityCx<'_, T> {
    #[must_use]
    pub fn focus_handle(&mut self) -> FocusHandle {
        let handle = FocusHandle::new();
        let id = handle.id();
        on_cleanup(move || {
            with_rt(|rt| rt.focus_stack.retain(|other| *other != id));
        });
        handle
    }

    pub fn focus(&mut self, handle: &FocusHandle) {
        let id = handle.id();
        with_rt(|rt| {
            crate::reactive::phase::assert_paint_safe(rt, "EntityCx::focus");
            rt.focus_stack.retain(|other| *other != id);
            rt.focus_stack.push(id);
        });
    }

    pub fn blur(&mut self, handle: &FocusHandle) {
        let id = handle.id();
        with_rt(|rt| {
            crate::reactive::phase::assert_paint_safe(rt, "EntityCx::blur");
            rt.focus_stack.retain(|other| *other != id);
        });
    }

    #[must_use]
    pub fn painting(&self) -> bool {
        with_rt(|rt| rt.phase == crate::reactive::DrawPhase::Paint)
    }

    #[must_use]
    pub fn focused(&self, handle: &FocusHandle) -> bool {
        let id = handle.id();
        with_rt(|rt| rt.focus_stack.contains(&id))
    }
}
