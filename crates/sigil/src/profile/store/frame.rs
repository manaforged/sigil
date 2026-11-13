use crate::profile::attr::Attribution;
use crate::profile::key::{BackendStatus, Cause, Counter, CounterValue, Phase, PhaseTime};
use crate::profile::site::Site;

kittest_pub! {
    /// Holds the recorded timings, counters, attributions, and GPU pass times of one frame.
    #[derive(Clone, Debug, Default)]
    pub struct Frame {
        /// The 0-based position of the frame among the frames closed since profiling began.
        pub index: u32,
        /// The highest-ranked input that arrived while the frame was open; `Idle` when none did.
        pub cause: Cause,
        /// The wall-clock time from the start of the frame to its close, in nanoseconds; 0 when the start was not recorded.
        pub wall_ns: u64,
        /// The time spent in each phase, in nanoseconds, with one entry per phase that ran.
        pub phases: Vec<PhaseTime>,
        /// The value of each counter that was touched in the frame.
        pub counts: Vec<CounterValue>,
        /// The renderer backend the frame began with; `None` when no frame was begun.
        pub backend: Option<BackendStatus>,
        /// The per-subject counts of each hotspot in the frame.
        pub hot: Vec<Attribution>,
        /// The label and total time, in nanoseconds, of each timed GPU pass.
        pub gpu_passes: Vec<(&'static str, u64)>,
        /// The time each effect and child list line took, slowest first once the frame closes.
        pub sites: Vec<Site>,
        /// When the frame closed, in microseconds since the profiler's origin.
        pub end_us: u64,
    }
}

impl Frame {
    kittest_pub! {
        /// Returns the value of `counter` in this frame.
        #[must_use]
        pub fn counter(&self, counter: Counter) -> u64 {
            self.counts
                .iter()
                .find(|(n, _)| *n == counter)
                .map_or(0, |(_, v)| *v)
        }
    }

    kittest_pub! {
        /// The frame time in ns: the wall time when recorded, else the sum of the top-level phases.
        #[must_use]
        pub fn total_ns(&self) -> u64 {
            if self.wall_ns > 0 {
                return self.wall_ns;
            }
            self.phases
                .iter()
                .filter(|(phase, _)| phase.is_top_level())
                .map(|(_, ns)| *ns)
                .sum()
        }
    }

    kittest_pub! {
        /// Returns the frame time minus the time spent waiting for the GPU or display.
        #[must_use]
        pub fn work_ns(&self) -> u64 {
            let wait = self
                .phases
                .iter()
                .filter(|(phase, _)| *phase == Phase::GpuWait)
                .map(|(_, ns)| *ns)
                .sum();
            self.total_ns().saturating_sub(wait)
        }
    }

    kittest_pub! {
        /// Returns the working phase that took longest, preferring nested phases over their parents.
        #[must_use]
        pub fn hottest(&self) -> Option<PhaseTime> {
            let wait = self.total_ns().saturating_sub(self.work_ns());
            let working = || {
                self.phases
                    .iter()
                    .copied()
                    .filter(|(phase, _)| *phase != Phase::GpuWait)
                    .map(move |(phase, ns)| match phase {
                        Phase::Paint | Phase::RenderCpu => (phase, ns.saturating_sub(wait)),
                        _ => (phase, ns),
                    })
            };
            working()
                .filter(|(phase, _)| !phase.is_top_level())
                .max_by_key(|(_, ns)| *ns)
                .or_else(|| working().max_by_key(|(_, ns)| *ns))
        }
    }
}
