use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct WindowHandle {
    inner: Box<dyn DisplayAndWindow>,
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

#[allow(
    clippy::redundant_pub_crate,
    reason = "pub(crate) is load-bearing: DisplayAndWindow shows up in \
              WindowHandle::into_inner's signature which is consumed by \
              the sibling `renderer` module."
)]
pub(crate) trait DisplayAndWindow: HasWindowHandle + HasDisplayHandle + Send + Sync {}

impl<T> DisplayAndWindow for T where T: HasWindowHandle + HasDisplayHandle + Send + Sync {}
