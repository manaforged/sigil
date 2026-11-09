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
    /// The window error kind.
    Window,
    /// The gpu error kind.
    Gpu,
    /// The font error kind.
    Font,
    /// The io error kind.
    Io,
    /// The task error kind.
    Task,
    /// The config error kind.
    Config,
    /// The other error kind.
    Other,
    /// The closed error kind.
    Closed,
    /// The empty shortcut error kind.
    EmptyShortcut,
    /// The unknown modifier error kind.
    UnknownModifier,
    /// The unknown key error kind.
    UnknownKey,
    /// The clipboard error kind.
    Clipboard,
    /// The svg error kind.
    Svg,
    /// The event loop error kind.
    EventLoop,
    /// The poisoned error kind.
    Poisoned,
    /// The already run error kind.
    AlreadyRun,
    /// The fetch error kind.
    Fetch,
    /// The http error kind.
    Http,
    /// The no adapter error kind.
    NoAdapter,
    /// The not initialized error kind.
    NotInitialized,
    /// The invalid viewport error kind.
    InvalidViewport,
    /// The software error kind.
    Software,
    /// The empty layer error kind.
    EmptyLayer,
    /// The layer too large error kind.
    LayerTooLarge,
    /// The layout error kind.
    Layout,
    /// The node missing error kind.
    NodeMissing,
    /// The not live error kind.
    NotLive,
    /// The not inline error kind.
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
        Self::new(err)
    }
}

impl Error {
    /// Creates an error.
    pub fn new<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::with_kind(ErrorKind::Other, err)
    }

    /// Wraps `err` in an error of the given kind.
    pub fn with_kind<E: StdError + Send + Sync + 'static>(kind: ErrorKind, err: E) -> Self {
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
        Self::with_kind(ErrorKind::Window, err)
    }

    #[cfg(any(
        test,
        feature = "desktop",
        feature = "terminal",
        feature = "pty",
        feature = "audio",
        feature = "clipboard"
    ))]
    pub(crate) fn io<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::with_kind(ErrorKind::Io, err)
    }

    #[cfg(any(feature = "desktop", feature = "pty", feature = "terminal"))]
    pub(crate) fn task<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::with_kind(ErrorKind::Task, err)
    }

    #[cfg(any(feature = "desktop", feature = "terminal"))]
    pub(crate) fn config<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::with_kind(ErrorKind::Config, err)
    }

    /// Returns the kind.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the node.
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
    #[cfg(all(
        feature = "scene",
        any(test, feature = "kittest", feature = "desktop", feature = "terminal")
    ))]
    pub(crate) fn gpu_device(err: wgpu::RequestDeviceError) -> Self {
        Self {
            kind: ErrorKind::Gpu,
            detail: Detail::Text(format!("failed to request GPU device: {err}")),
            source: Some(Arc::new(err)),
        }
    }

    #[cfg(all(
        feature = "scene",
        any(test, feature = "kittest", feature = "desktop", feature = "terminal")
    ))]
    pub(crate) fn gpu_surface(err: wgpu::CreateSurfaceError) -> Self {
        Self {
            kind: ErrorKind::Gpu,
            detail: Detail::Text(format!("failed to create GPU surface: {err}")),
            source: Some(Arc::new(err)),
        }
    }

    #[cfg(feature = "desktop")]
    pub(crate) fn event_loop(err: winit::error::EventLoopError) -> Self {
        Self {
            kind: ErrorKind::EventLoop,
            detail: Detail::Text(format!("winit event loop error: {err}")),
            source: Some(Arc::new(err)),
        }
    }
}
