use crate::profile::attr::Attribution;
use crate::profile::key::{BackendStatus, Cause, Counter, CounterValue, Phase, PhaseTime};
use crate::profile::site::Site;

kittest_pub! {
    /// Holds the recorded timings, counters, attributions, and GPU pass times of one frame.
    #[derive(Clone, Debug, Default)]
    pub struct Frame {
        /// The index.
        pub index: u32,
        /// The cause.
        pub cause: Cause,
        /// The wall ns.
        pub wall_ns: u64,
        /// The phases.
        pub phases: Vec<PhaseTime>,
        /// The counts.
        pub counts: Vec<CounterValue>,
        /// The backend, if any.
        pub backend: Option<BackendStatus>,
        /// The hot.
        pub hot: Vec<Attribution>,
        /// The gpu passes.
        pub gpu_passes: Vec<(&'static str, u64)>,
        /// The time each effect and child list line took, slowest first once the frame closes.
        pub sites: Vec<Site>,
        /// The end us.
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
        /// Returns the total ns.
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
