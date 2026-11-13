use core::panic::Location;

use web_time::Instant;

use crate::profile::key::SiteKind;
use crate::profile::store;

kittest_pub! {
    /// The time one source line spent in a frame.
    #[derive(Clone, Copy, Debug)]
    pub struct Site {
        /// The line that created the effect or child list.
        pub location: &'static Location<'static>,
        /// Whether the line ran as an effect, a mount, or a message.
        pub kind: SiteKind,
        /// The time spent in this site, minus nested sites.
        pub self_ns: u64,
        /// The time spent in this site, with nested sites.
        pub total_ns: u64,
        /// How many times this site ran in the frame.
        pub runs: u32,
    }
}

/// Times one run of a site from creation until drop.
pub(crate) struct SiteSpan {
    open: Option<(&'static Location<'static>, SiteKind, Instant)>,
}

pub(crate) fn site(location: &'static Location<'static>, kind: SiteKind) -> SiteSpan {
    SiteSpan {
        open: store::enabled().then(|| {
            store::site_enter();
            (location, kind, Instant::now())
        }),
    }
}

impl Drop for SiteSpan {
    fn drop(&mut self) {
        if let Some((location, kind, start)) = self.open.take() {
            let total_ns = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
            store::site_exit(location, kind, start, total_ns);
        }
    }
}
