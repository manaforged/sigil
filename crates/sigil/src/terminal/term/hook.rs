use super::{owns_terminal, restore};

type PanicHook = Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync + 'static>;

pub(crate) struct HookGuard {
    previous: Option<PanicHook>,
}

impl HookGuard {
    pub(crate) fn install() -> Self {
        let previous: std::sync::Arc<PanicHook> = std::sync::Arc::new(std::panic::take_hook());
        let chained = std::sync::Arc::clone(&previous);
        std::panic::set_hook(Box::new(move |info| {
            if owns_terminal() {
                restore();
            }
            chained(info);
        }));
        Self {
            previous: Some(Box::new(move |info| previous(info))),
        }
    }
}

impl Drop for HookGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            std::panic::set_hook(previous);
        }
    }
}
