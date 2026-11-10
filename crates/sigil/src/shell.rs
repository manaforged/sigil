pub(crate) mod app;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod composition;
pub(crate) mod dpi;
pub(crate) mod event_map;
pub(crate) mod exit_hook;
pub(crate) mod key_map;
#[cfg(windows)]
pub(crate) mod session_end;
#[allow(clippy::module_inception)]
pub(crate) mod shell;
pub(crate) mod waker;

pub use shell::WinitShell;
#[cfg(any(test, feature = "kittest"))]
pub use waker::WAKE_DELIVERED_COUNT;
pub use waker::{WakeEvent, WinitWaker};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct WinitWindowHandle(u64);

impl WinitWindowHandle {
    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub const fn id(self) -> u64 {
        self.0
    }

    pub(crate) const fn from_raw(id: u64) -> Self {
        Self(id)
    }
}
