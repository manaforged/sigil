#[derive(Clone, Copy, Debug)]
pub(crate) enum Event {
    Complete {
        name: &'static str,
        cat: &'static str,
        tid: u32,
        start_us: u64,
        dur_us: u64,
    },
    Instant {
        name: &'static str,
        cat: &'static str,
        ts_us: u64,
    },
    Counter {
        name: &'static str,
        ts_us: u64,
        value: u64,
    },
    FlowStart {
        name: &'static str,
        id: u64,
        ts_us: u64,
    },
    FlowFinish {
        name: &'static str,
        id: u64,
        ts_us: u64,
    },
}
