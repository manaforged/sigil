use core::fmt;
use std::error::Error as StdError;

pub type AppResult<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: Box<dyn StdError + Send + Sync + 'static>,
}

impl Error {
    pub fn new<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self {
            inner: Box::new(err),
        }
    }

    #[must_use]
    pub fn source_error(&self) -> &(dyn StdError + Send + Sync + 'static) {
        &*self.inner
    }
}

impl From<crate::layout::LayoutError> for Error {
    fn from(err: crate::layout::LayoutError) -> Self {
        Self::new(err)
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
impl From<crate::renderer::RendererError> for Error {
    fn from(err: crate::renderer::RendererError) -> Self {
        Self::new(err)
    }
}
