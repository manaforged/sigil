use web_time::Instant;

use crate::profile::key::Phase;
use crate::profile::store;

pub struct Span {
    phase: Phase,
    start_us: Option<u64>,
    start: Option<Instant>,
}

impl Span {
    pub(crate) fn new(phase: Phase) -> Self {
        if store::enabled() {
            Self {
                phase,
                start_us: Some(store::now_us()),
                start: Some(Instant::now()),
            }
        } else {
            Self {
                phase,
                start_us: None,
                start: None,
            }
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let (Some(start_us), Some(start)) = (self.start_us.take(), self.start.take()) else {
            return;
        };
        store::complete(
            self.phase,
            start_us,
            u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX),
        );
    }
}
