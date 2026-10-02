use std::env;
use std::sync::OnceLock;

use crate::profile::key::{Counter, Hotspot, Phase, accumulate};
use crate::profile::store::{self, FRAME_BUDGET_NS};

pub const MAX_SUBJECTS: usize = 64;
pub const OTHER_SUBJECT: &str = "(other)";
pub const BUDGET_ENV: &str = "SIGIL_PROFILE_BUDGET_MS";
pub const DEFAULT_BUDGET_MS: f64 = 8.33;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribution {
    pub hotspot: Hotspot,
    pub subject: String,
    pub count: u64,
}

pub(crate) fn attribute(hotspot: Hotspot, n: u64, subject: impl FnOnce() -> String) {
    if !store::enabled() {
        return;
    }
    let subject = subject();
    store::with_open(|frame| {
        let hot = &mut frame.hot;
        if let Some(entry) = hot
            .iter_mut()
            .find(|a| a.hotspot == hotspot && a.subject == subject)
        {
            entry.count = entry.count.saturating_add(n);
            return;
        }
        let distinct = hot.iter().filter(|a| a.hotspot == hotspot).count();
        let subject = if distinct + 1 >= MAX_SUBJECTS {
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

pub(crate) fn gpu_pass(label: &'static str, ns: u64) {
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
