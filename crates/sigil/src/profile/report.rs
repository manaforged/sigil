use std::cmp::Ordering;
use std::fmt::{Display, Write as _};

use crate::profile::attr::budget_ns;
use crate::profile::key::{Cause, Phase};

use crate::profile::report_why::{write_bottlenecks, write_gpu_passes, write_why};
use crate::profile::store::{self, Frame};

const CAUSE_PHASES: usize = 7;

pub(crate) fn summary() -> String {
    let frames = store::frames();
    if frames.is_empty() {
        return String::from("sigil-profile: no frames");
    }
    let budget = budget_ns();
    let mut out = format!("sigil-profile  (ms, budget {:.2}ms)\n", ns_ms(budget));
    let _ = writeln!(
        out,
        "{:<14} {:>8} {:>8} {:>8} {:>8}",
        "phase", "p50", "p95", "max", "last"
    );
    for phase in distinct(frames.iter().flat_map(|frame| keys(&frame.phases))) {
        write_row(
            &mut out,
            phase,
            frames.iter().map(|f| value_ms(&f.phases, phase)),
        );
    }
    write_row(
        &mut out,
        "frame",
        frames.iter().map(|f| ns_ms(f.total_ns())),
    );
    write_interact_rows(&mut out, &frames);
    let counters = distinct(frames.iter().flat_map(|frame| keys(&frame.counts)));
    if !counters.is_empty() {
        out.push_str("\ncounts\n");
    }
    for counter in counters {
        write_row(
            &mut out,
            counter,
            frames.iter().map(|f| value_u64(&f.counts, counter)),
        );
    }
    write_gpu_passes(&mut out, &frames);
    write_why(&mut out, &frames);
    write_bottlenecks(&mut out, &frames, budget);
    let over = frames
        .iter()
        .filter(|frame| frame.total_ns() > budget)
        .count();
    let _ = write!(out, "{} frames", frames.len());
    if over > 0 {
        let _ = write!(out, ", {over} over {:.2}ms", ns_ms(budget));
    }
    out.push('\n');
    out
}

fn write_row<T: Copy + Default + Display + PartialOrd>(
    out: &mut String,
    name: impl Display,
    samples: impl Iterator<Item = T>,
) {
    let [p50, p95, max, last] = spread(samples.collect());
    let _ = writeln!(
        out,
        "{name:<14} {p50:>8.3} {p95:>8.3} {max:>8.3} {last:>8.3}"
    );
}

fn write_interact_rows(out: &mut String, frames: &[Frame]) {
    let causes = distinct(
        frames
            .iter()
            .map(|frame| frame.cause)
            .filter(|&cause| cause != Cause::Idle),
    );
    if causes.is_empty() {
        return;
    }
    out.push_str("\ninteractions     n      p50      p95      max     last\n");
    for cause in causes {
        let picked: Vec<&Frame> = frames.iter().filter(|frame| frame.cause == cause).collect();
        let [p50, p95, max, last] = spread(picked.iter().map(|f| ns_ms(f.total_ns())).collect());
        let _ = writeln!(
            out,
            "{cause:<14} {:>8} {p50:>8.3} {p95:>8.3} {max:>8.3} {last:>8.3}",
            picked.len(),
        );
        write_cause_phases(out, &picked);
    }
}

fn write_cause_phases(out: &mut String, picked: &[&Frame]) {
    let p50 = |sample: &dyn Fn(&Frame) -> f64| {
        spread(picked.iter().map(|&frame| sample(frame)).collect())[0]
    };
    let mut phases: Vec<(Phase, f64)> =
        distinct(picked.iter().flat_map(|frame| keys(&frame.phases)))
            .into_iter()
            .map(|phase| (phase, p50(&|frame| value_ms(&frame.phases, phase))))
            .filter(|(_, ms)| *ms > 0.0)
            .collect();
    phases.sort_by(|a, b| b.1.total_cmp(&a.1));
    let cpu = p50(&|frame| ns_ms(frame.total_ns()) - value_ms(&frame.phases, Phase::GpuWait));
    let _ = write!(out, "  p50 cpu {cpu:.3} |");
    for (phase, ms) in phases.iter().take(CAUSE_PHASES) {
        let _ = write!(out, " {phase} {ms:.3}");
    }
    out.push('\n');
}

fn keys<K: Copy>(list: &[(K, u64)]) -> impl Iterator<Item = K> + '_ {
    list.iter().map(|&(key, _)| key)
}

pub(super) fn distinct<K: PartialEq>(items: impl Iterator<Item = K>) -> Vec<K> {
    let mut seen = Vec::new();
    for item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    seen
}

pub(super) fn spread<T: Copy + Default + PartialOrd>(mut samples: Vec<T>) -> [T; 4] {
    let last = samples.last().copied().unwrap_or_default();
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    [
        pct(&samples, 50),
        pct(&samples, 95),
        pct(&samples, 100),
        last,
    ]
}

fn value_ms<K: Copy + PartialEq>(list: &[(K, u64)], name: K) -> f64 {
    ns_ms(value_u64(list, name))
}

fn value_u64<K: Copy + PartialEq>(list: &[(K, u64)], name: K) -> u64 {
    list.iter()
        .find(|(n, _)| *n == name)
        .map_or(0, |(_, ns)| *ns)
}

#[allow(clippy::cast_precision_loss, reason = "ns → ms display only")]
pub(super) fn ns_ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

fn pct<T: Copy + Default>(sorted: &[T], p: usize) -> T {
    sorted
        .get(sorted.len().saturating_sub(1) * p / 100)
        .copied()
        .unwrap_or_default()
}
