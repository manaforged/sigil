use core::fmt;
use std::error::Error as StdError;
use std::sync::Arc;

use crate::kernel::NodeId;

/// Result of the application functions.
pub type AppResult<T = ()> = core::result::Result<T, Error>;

type ErrorSource = Arc<dyn StdError + Send + Sync + 'static>;

/// Class of an `Error`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// The window host could not start or could not create a window.
    Window,
    /// A GPU device, surface, or frame could not be acquired.
    Gpu,
    /// Font data is not valid.
    Font,
    /// An input or output operation failed.
    Io,
    /// A task could not start, or it panicked.
    Task,
    /// A configuration is not valid, or this build does not support it.
    Config,
    /// A failure that fits no other kind.
    Other,
    /// The event loop or the main-thread dispatcher is closed.
    Closed,
    /// A shortcut string is empty.
    EmptyShortcut,
    /// A shortcut string names an unknown modifier.
    UnknownModifier,
    /// A shortcut string names an unknown key.
    UnknownKey,
    /// No clipboard is installed, or the clipboard does not accept the data.
    Clipboard,
    /// The window event loop failed.
    EventLoop,
    /// A mutex is poisoned.
    Poisoned,
    /// The window host was started twice.
    AlreadyRun,
    /// A network request could not be made or did not complete.
    Fetch,
    /// A request failed with an HTTP status from the server.
    Http,
    /// No compatible GPU adapter was found.
    NoAdapter,
    /// The renderer has not picked a GPU device yet.
    NotInitialized,
    /// A viewport width or height is not greater than zero.
    InvalidViewport,
    /// The software renderer failed.
    Software,
    /// An offscreen layer has a zero extent.
    EmptyLayer,
    /// An offscreen layer is larger than the device limit.
    LayerTooLarge,
    /// Layout failed, for example because the tree holds a stale node id.
    Layout,
    /// A node is missing from the layout; `Error::node` returns its id.
    NodeMissing,
    /// No terminal host is live on this thread, or the terminal is released.
    NotLive,
    /// The terminal host is not inline.
    NotInline,
}

#[derive(Debug, Clone)]
enum Detail {
    Wrapped,
    Static(&'static str),
    Text(String),
    Node(NodeId),
}

impl Detail {
    fn text(&self) -> Option<&str> {
        match self {
            Self::Static(message) => Some(message),
            Self::Text(message) => Some(message),
            Self::Wrapped | Self::Node(_) => None,
        }
    }
}

#[doc = include_str!("../docs/error.md")]
#[derive(Debug, Clone)]
pub struct Error {
    kind: ErrorKind,
    detail: Detail,
    source: Option<ErrorSource>,
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        if self.kind != other.kind {
            return false;
        }
        match (&self.detail, &other.detail) {
            (Detail::Node(a), Detail::Node(b)) => a == b,
            (Detail::Wrapped, Detail::Wrapped) => match (&self.source, &other.source) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            },
            (a, b) => a.text().is_some() && a.text() == b.text(),
        }
    }
}

impl Eq for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::new(ErrorKind::Io, err)
    }
}

impl Error {
    /// Wraps `err` in an error of the given kind.
    pub fn new<E: StdError + Send + Sync + 'static>(kind: ErrorKind, err: E) -> Self {
        Self {
            kind,
            detail: Detail::Wrapped,
            source: Some(Arc::new(err)),
        }
    }

    /// Build an error with a kind and a message, for example from a `Clipboard` back end with `ErrorKind::Clipboard`.
    pub fn message(kind: ErrorKind, message: impl fmt::Display) -> Self {
        Self {
            kind,
            detail: Detail::Text(message.to_string()),
            source: None,
        }
    }

    pub(crate) const fn fixed(kind: ErrorKind, message: &'static str) -> Self {
        Self {
            kind,
            detail: Detail::Static(message),
            source: None,
        }
    }

    pub(crate) fn layout(message: impl fmt::Display) -> Self {
        Self::message(ErrorKind::Layout, format_args!("layout error: {message}"))
    }

    pub(crate) const fn node_missing(id: NodeId) -> Self {
        Self {
            kind: ErrorKind::NodeMissing,
            detail: Detail::Node(id),
            source: None,
        }
    }

    #[cfg(feature = "desktop")]
    pub(crate) fn window<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::new(ErrorKind::Window, err)
    }

    #[cfg(any(app_host, feature = "pty", feature = "audio", feature = "clipboard"))]
    pub(crate) fn io<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::new(ErrorKind::Io, err)
    }

    #[cfg(any(app_host, feature = "pty"))]
    pub(crate) fn task<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::new(ErrorKind::Task, err)
    }

    #[cfg(app_host)]
    pub(crate) fn config<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::new(ErrorKind::Config, err)
    }

    /// The class of this error; branch on it, not on the message.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The missing node of an `ErrorKind::NodeMissing` error; `None` for every other error.
    #[must_use]
    pub const fn node(&self) -> Option<NodeId> {
        match self.detail {
            Detail::Node(id) => Some(id),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.detail {
            Detail::Wrapped => self.source.as_ref().map_or(Ok(()), |inner| inner.fmt(f)),
            Detail::Node(id) => write!(f, "layout node missing: {id:?}"),
            detail => f.write_str(detail.text().unwrap_or_default()),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        let inner = self.source.as_deref()?;
        match self.detail {
            Detail::Wrapped => inner.source(),
            _ => Some(inner),
        }
    }
}

impl Error {
    #[cfg(scene_host)]
    pub(crate) fn gpu_device(err: wgpu::RequestDeviceError) -> Self {
        Self {
            kind: ErrorKind::Gpu,
            detail: Detail::Static("failed to request GPU device"),
            source: Some(Arc::new(err)),
        }
    }

    #[cfg(scene_host)]
    pub(crate) fn gpu_surface(err: wgpu::CreateSurfaceError) -> Self {
        Self {
            kind: ErrorKind::Gpu,
            detail: Detail::Static("failed to create GPU surface"),
            source: Some(Arc::new(err)),
        }
    }

    #[cfg(feature = "desktop")]
    pub(crate) fn event_loop(err: winit::error::EventLoopError) -> Self {
        Self {
            kind: ErrorKind::EventLoop,
            detail: Detail::Static("window event loop failed"),
            source: Some(Arc::new(err)),
        }
    }
}
