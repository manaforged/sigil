use web_time::Instant;

use crate::profile::key::Phase;
use crate::profile::store;

pub struct Span {
    phase: Phase,
    start: Option<(u64, Instant)>,
}

impl Span {
    pub(crate) fn new(phase: Phase) -> Self {
        Self {
            phase,
            start: store::enabled().then(|| (store::now_us(), Instant::now())),
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some((start_us, start)) = self.start.take() {
            let elapsed = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
            store::complete(self.phase, start_us, elapsed);
        }
    }
}
