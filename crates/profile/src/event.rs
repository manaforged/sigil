//! Chrome / Perfetto trace events.

/// One recorded event.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Event {
    /// Complete interval (`ph: X`).
    Complete {
        name: &'static str,
        cat: &'static str,
        tid: u32,
        start_us: u64,
        dur_us: u64,
    },
    /// Instant (`ph: i`).
    Instant {
        name: &'static str,
        cat: &'static str,
        ts_us: u64,
    },
    /// Counter sample (`ph: C`).
    Counter {
        name: &'static str,
        ts_us: u64,
        value: u64,
    },
    /// Flow start (`ph: s`) — input to the frame that presented it.
    FlowStart {
        name: &'static str,
        id: u64,
        ts_us: u64,
    },
    /// Flow finish (`ph: f`).
    FlowFinish {
        name: &'static str,
        id: u64,
        ts_us: u64,
    },
}
