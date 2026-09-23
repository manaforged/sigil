//! The winit shell: window loop and event translation.
use winit::error::EventLoopError;

pub(crate) mod app;
pub(crate) mod dpi;
pub(crate) mod event_map;
pub(crate) mod key_map;
#[allow(clippy::module_inception)]
pub(crate) mod shell;
pub(crate) mod waker;

pub use shell::WinitShell;
pub use waker::{WAKE_DELIVERED_COUNT, WakeEvent, WinitWaker};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct WinitWindowHandle(u64);

impl WinitWindowHandle {
    #[must_use]
    pub const fn id(self) -> u64 {
        self.0
    }

    pub(crate) const fn from_raw(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug)]
pub enum ShellError {
    EventLoop(EventLoopError),
    EventLoopClosed,
    WakerPoisoned,
    AlreadyRun,
    Window(String),
}

impl core::fmt::Display for ShellError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EventLoop(err) => write!(f, "winit event loop error: {err}"),
            Self::EventLoopClosed => f.write_str("event loop already exited"),
            Self::WakerPoisoned => f.write_str("waker mutex poisoned"),
            Self::AlreadyRun => f.write_str("shell run called twice"),
            Self::Window(err) => write!(f, "window creation failed: {err}"),
        }
    }
}

impl std::error::Error for ShellError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EventLoop(err) => Some(err),
            _ => None,
        }
    }
}

impl From<EventLoopError> for ShellError {
    fn from(err: EventLoopError) -> Self {
        Self::EventLoop(err)
    }
}
