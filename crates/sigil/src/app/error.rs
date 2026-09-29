use core::fmt;
use std::error::Error as StdError;

pub type AppResult<T = ()> = core::result::Result<T, Error>;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    Window,
    Gpu,
    Font,
    Io,
    Task,
    Config,
    Other,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    inner: Box<dyn StdError + Send + Sync + 'static>,
}

impl Error {
    pub fn new<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::with_kind(ErrorKind::Other, err)
    }

    pub fn with_kind<E: StdError + Send + Sync + 'static>(kind: ErrorKind, err: E) -> Self {
        Self {
            kind,
            inner: Box::new(err),
        }
    }

    #[cfg(feature = "desktop")]
    pub(crate) fn window<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::with_kind(ErrorKind::Window, err)
    }

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

    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    #[must_use]
    pub fn source_error(&self) -> &(dyn StdError + Send + Sync + 'static) {
        &*self.inner
    }
}

impl From<crate::layout::LayoutError> for Error {
    fn from(err: crate::layout::LayoutError) -> Self {
        Self::with_kind(ErrorKind::Other, err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner.source()
    }
}

#[cfg(feature = "desktop")]
impl From<crate::renderer::RendererError> for Error {
    fn from(err: crate::renderer::RendererError) -> Self {
        Self::with_kind(ErrorKind::Gpu, err)
    }
}
