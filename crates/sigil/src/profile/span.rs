use web_time::Instant;

use crate::profile::key::Phase;
use crate::profile::store;

/// Measures one phase from creation until drop.
#[derive(Debug)]
pub struct Span {
    phase: Phase,
    start: Option<Instant>,
}

impl Span {
    pub(crate) fn new(phase: Phase) -> Self {
        Self {
            phase,
            start: store::enabled().then(Instant::now),
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some(start) = self.start.take() {
            let elapsed = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
            store::complete(self.phase, start, elapsed);
        }
    }
}
