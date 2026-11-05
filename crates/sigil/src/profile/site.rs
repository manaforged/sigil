use core::panic::Location;

use web_time::Instant;

use crate::profile::store;

/// What a profiled site was doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SiteKind {
    /// An effect, derived value or child list closure ran.
    Effect,
    /// A child list built, moved or removed its nodes.
    Mount,
    /// A task message handler ran on the UI thread.
    Message,
}

impl SiteKind {
    /// Returns the name this kind has in profile output.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Effect => "effect",
            Self::Mount => "mount",
            Self::Message => "message",
        }
    }
}

/// The time one source line spent in a frame.
#[derive(Clone, Copy, Debug)]
pub struct Site {
    /// The line that created the effect or child list.
    pub location: &'static Location<'static>,
    /// The kind.
    pub kind: SiteKind,
    /// The time spent in this site, minus nested sites.
    pub self_ns: u64,
    /// The time spent in this site, with nested sites.
    pub total_ns: u64,
    /// The runs.
    pub runs: u32,
}

/// Times one run of a site from creation until drop.
pub(crate) struct SiteSpan {
    open: Option<(&'static Location<'static>, SiteKind, u64, Instant)>,
}

pub(crate) fn site(location: &'static Location<'static>, kind: SiteKind) -> SiteSpan {
    SiteSpan {
        open: store::enabled().then(|| {
            store::site_enter();
            (location, kind, store::now_us(), Instant::now())
        }),
    }
}

impl Drop for SiteSpan {
    fn drop(&mut self) {
        if let Some((location, kind, start_us, start)) = self.open.take() {
            let total_ns = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
            store::site_exit(location, kind, start_us, total_ns);
        }
    }
}
