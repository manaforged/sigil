//! `sigil::Error` — unified error type + `sigil::AppResult` alias.

use core::fmt;
use std::error::Error as StdError;

/// `Result<T, Error>` from `launch`, clipboard, audio, and PTY.
pub type AppResult<T> = core::result::Result<T, Error>;

/// The one error type returned by [`crate::launch`] and anything user code
/// calls through it. Wraps boxed errors from shell / renderer / layout so
/// callers get one clean `?` chain.
#[derive(Debug)]
pub struct Error {
    inner: Box<dyn StdError + Send + Sync + 'static>,
}

impl Error {
    /// Wrap any `std::error::Error` into a [`crate::Error`].
    pub fn new<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self {
            inner: Box::new(err),
        }
    }

    /// Borrow the inner error for inspection.
    #[must_use]
    pub fn source_error(&self) -> &(dyn StdError + Send + Sync + 'static) {
        &*self.inner
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

impl From<Box<dyn StdError + Send + Sync + 'static>> for Error {
    fn from(inner: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        Self { inner }
    }
}

#[cfg(feature = "desktop")]
impl From<sigil_renderer::RendererError> for Error {
    fn from(err: sigil_renderer::RendererError) -> Self {
        Self::new(err)
    }
}
