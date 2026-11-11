#[cfg(feature = "terminal")]
#[derive(Clone, Default)]
pub(crate) struct TerminalHooks(
    pub(super) std::rc::Rc<std::cell::RefCell<Option<crate::terminal::Hooks<'static>>>>,
);

#[cfg(feature = "terminal")]
impl TerminalHooks {
    pub(crate) fn take(&self) -> crate::terminal::Hooks<'static> {
        self.0.borrow_mut().take().unwrap_or_default()
    }
}

#[cfg(feature = "terminal")]
impl std::fmt::Debug for TerminalHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TerminalHooks")
    }
}

#[cfg(any(feature = "desktop", feature = "terminal"))]
#[derive(Clone, Default)]
pub(crate) struct ExitHook(pub(crate) Option<std::rc::Rc<dyn Fn()>>);

#[cfg(any(feature = "desktop", feature = "terminal"))]
impl std::fmt::Debug for ExitHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExitHook")
    }
}

#[cfg(feature = "desktop")]
#[derive(Clone, Default)]
pub(crate) struct CloseHook(pub(super) Option<std::rc::Rc<dyn Fn() -> crate::app::CloseAction>>);

#[cfg(feature = "desktop")]
impl CloseHook {
    pub(crate) fn decide(&self) -> crate::app::CloseAction {
        self.0
            .as_ref()
            .map_or(crate::app::CloseAction::Close, |hook| hook())
    }
}

#[cfg(feature = "desktop")]
impl std::fmt::Debug for CloseHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CloseHook")
    }
}
