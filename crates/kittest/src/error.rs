use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;

type Source = Arc<dyn StdError + Send + Sync + 'static>;

/// Class of an `Error`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// No wgpu adapter is available.
    NoAdapter,
    /// The wgpu device could not be opened or is missing.
    Device,
    /// Layout of the tree under test failed.
    Layout,
    /// The offscreen render failed.
    Render,
    /// The rendered pixels could not be read back.
    Readback,
    /// The frame did not settle within `SETTLE_SAFETY_CAP` frames.
    Unsettled,
}

impl ErrorKind {
    const fn label(self) -> &'static str {
        match self {
            Self::NoAdapter => "no wgpu adapter; set SIGIL_ALLOW_NO_GPU=1 to skip GPU checks",
            Self::Device => "wgpu device request failed",
            Self::Layout => "layout failed",
            Self::Render => "offscreen render failed",
            Self::Readback => "pixel readback failed",
            Self::Unsettled => "the frame did not settle",
        }
    }
}

#[derive(Debug, Clone)]
enum Detail {
    Kind,
    Text(String),
    Unsettled { frames: u32, active: usize },
}

/// Failure of a harness layout, render, or pixel readback.
#[derive(Debug, Clone)]
pub struct Error {
    kind: ErrorKind,
    detail: Detail,
    source: Option<Source>,
}

impl Error {
    pub(crate) fn caused(kind: ErrorKind, source: impl StdError + Send + Sync + 'static) -> Self {
        Self {
            kind,
            detail: Detail::Kind,
            source: Some(Arc::new(source)),
        }
    }

    pub(crate) fn text(kind: ErrorKind, text: impl fmt::Display) -> Self {
        Self {
            kind,
            detail: Detail::Text(text.to_string()),
            source: None,
        }
    }

    pub(crate) const fn unsettled(frames: u32, active: usize) -> Self {
        Self {
            kind: ErrorKind::Unsettled,
            detail: Detail::Unsettled { frames, active },
            source: None,
        }
    }

    pub(crate) fn chain(&self) -> String {
        let mut text = self.to_string();
        let mut cause = StdError::source(self);
        while let Some(inner) = cause {
            text.push_str(": ");
            text.push_str(&inner.to_string());
            cause = inner.source();
        }
        text
    }

    /// Returns the kind.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Reports whether no wgpu adapter is available.
    #[must_use]
    pub const fn is_no_adapter(&self) -> bool {
        matches!(self.kind, ErrorKind::NoAdapter)
    }

    /// Reports whether the frame did not settle within `SETTLE_SAFETY_CAP` frames.
    #[must_use]
    pub const fn is_unsettled(&self) -> bool {
        matches!(self.kind, ErrorKind::Unsettled)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = self.kind.label();
        match &self.detail {
            Detail::Kind => f.write_str(label),
            Detail::Text(text) => write!(f, "{label}: {text}"),
            Detail::Unsettled { frames, active: 0 } => write!(
                f,
                "the frame stayed dirty for SETTLE_SAFETY_CAP ({frames}) frames with no \
                 finite animation running, so an effect or a signal write under test \
                 likely marks every frame dirty"
            ),
            Detail::Unsettled { frames, active } => write!(
                f,
                "animation runtime did not settle within SETTLE_SAFETY_CAP ({frames}) frames; \
                 {active} finite animators remain, so a widget under test likely \
                 retargets every frame"
            ),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        let source = self.source.as_deref()?;
        Some(source)
    }
}
