use std::cell::RefCell;

pub(crate) type ExitHook = Box<dyn FnOnce()>;

thread_local! {
    static HOOK: RefCell<Option<ExitHook>> = const { RefCell::new(None) };
}

pub(crate) fn set(hook: ExitHook) {
    HOOK.with(|slot| *slot.borrow_mut() = Some(hook));
}

#[cfg(any(windows, test))]
pub(crate) fn pending() -> bool {
    HOOK.with(|slot| slot.borrow().is_some())
}

pub(crate) fn run() {
    let hook = HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(test)]
#[path = "exit_hook_tests.rs"]
mod tests;
