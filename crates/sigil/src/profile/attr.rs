use std::env;
use std::sync::OnceLock;

use crate::profile::key::Hotspot;
use crate::profile::store::{self, FRAME_BUDGET_NS};

/// The most subjects one hotspot tracks before it folds the rest.
const MAX_SUBJECTS: usize = 64;
/// The subject name that holds folded counts.
const OTHER_SUBJECT: &str = "(other)";
/// The environment variable that overrides the frame budget.
const BUDGET_ENV: &str = "SIGIL_PROFILE_BUDGET_MS";
/// The default attribution budget in milliseconds.
const DEFAULT_BUDGET_MS: f64 = 8.33;

/// The counts per subject of one hotspot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribution {
    /// The hotspot.
    pub hotspot: Hotspot,
    /// The subject.
    pub subject: String,
    /// The count.
    pub count: u64,
}

pub(crate) fn attribute(hotspot: Hotspot, n: u64, subject: impl FnOnce() -> String) {
    if !store::enabled() {
        return;
    }
    let subject = subject();
    store::with_open(|frame| {
        let hot = &mut frame.hot;
        let known = hot
            .iter()
            .any(|a| a.hotspot == hotspot && a.subject == subject);
        let distinct = hot.iter().filter(|a| a.hotspot == hotspot).count();
        let subject = if !known && distinct + 1 >= MAX_SUBJECTS {
            String::from(OTHER_SUBJECT)
        } else {
            subject
        };
        if let Some(entry) = hot
            .iter_mut()
            .find(|a| a.hotspot == hotspot && a.subject == subject)
        {
            entry.count = entry.count.saturating_add(n);
        } else {
            hot.push(Attribution {
                hotspot,
                subject,
                count: n,
            });
        }
    });
}

#[cfg(any(
    test,
    all(
        feature = "scene",
        any(feature = "desktop", feature = "kittest", feature = "terminal")
    )
))]
pub(crate) fn gpu_pass(label: &'static str, ns: u64) {
    use crate::profile::key::{Counter, Phase, accumulate};

    store::with_open(|frame| {
        accumulate(&mut frame.gpu_passes, label, ns);
        accumulate(&mut frame.phases, Phase::Gpu, ns);
        accumulate(&mut frame.counts, Counter::GpuTimedPasses, 1);
    });
}

pub(crate) fn budget_ns() -> u64 {
    static BUDGET: OnceLock<u64> = OnceLock::new();
    *BUDGET.get_or_init(|| {
        env::var(BUDGET_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<f64>().ok())
            .filter(|ms| ms.is_finite() && *ms > 0.0)
            .or(Some(DEFAULT_BUDGET_MS))
            .map_or(FRAME_BUDGET_NS, ms_to_ns)
    })
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "positive finite budget in ms"
)]
fn ms_to_ns(ms: f64) -> u64 {
    (ms * 1_000_000.0).round() as u64
}
