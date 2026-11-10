use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct WindowHandle {
    inner: Box<dyn DisplayAndWindow>,
}

impl core::fmt::Debug for WindowHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WindowHandle").finish_non_exhaustive()
    }
}

impl WindowHandle {
    pub(crate) fn into_inner(self) -> Box<dyn DisplayAndWindow> {
        self.inner
    }
}

pub fn into_window_handle<W>(window: W) -> WindowHandle
where
    W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
{
    WindowHandle {
        inner: Box::new(window),
    }
}

pub(crate) trait DisplayAndWindow: HasWindowHandle + HasDisplayHandle + Send + Sync {}

impl<T> DisplayAndWindow for T where T: HasWindowHandle + HasDisplayHandle + Send + Sync {}
