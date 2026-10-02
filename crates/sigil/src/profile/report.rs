use std::fmt::Write as _;

use std::fmt::Display;

use crate::profile::attr::budget_ns;
use crate::profile::key::{Cause, Counter, Phase};

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
    write_named_rows(&mut out, &frames, phases_of);
    write_total_row(&mut out, &frames);
    write_interact_rows(&mut out, &frames);
    if !collect_names(&frames, counts_of).is_empty() {
        out.push_str("\ncounts\n");
        write_count_rows(&mut out, &frames);
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

type Pick<K> = fn(&Frame) -> &[(K, u64)];

fn phases_of(frame: &Frame) -> &[(Phase, u64)] {
    &frame.phases
}

fn counts_of(frame: &Frame) -> &[(Counter, u64)] {
    &frame.counts
}

fn write_named_rows<K: Copy + PartialEq + Display>(
    out: &mut String,
    frames: &[Frame],
    pick: Pick<K>,
) {
    for name in collect_names(frames, pick) {
        let last = frames
            .last()
            .map_or(0.0, |frame| value_ms(pick(frame), name));
        let mut samples: Vec<f64> = frames.iter().map(|f| value_ms(pick(f), name)).collect();
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let _ = writeln!(
            out,
            "{name:<14} {:>8.3} {:>8.3} {:>8.3} {last:>8.3}",
            pct(&samples, 50),
            pct(&samples, 95),
            samples.last().copied().unwrap_or(0.0),
        );
    }
}

fn write_interact_rows(out: &mut String, frames: &[Frame]) {
    let mut names: Vec<Cause> = Vec::new();
    for frame in frames {
        if frame.cause != Cause::Idle && !names.contains(&frame.cause) {
            names.push(frame.cause);
        }
    }
    if names.is_empty() {
        return;
    }
    out.push_str("\ninteractions     n      p50      p95      max     last\n");
    for name in names {
        let samples: Vec<f64> = frames
            .iter()
            .filter(|frame| frame.cause == name)
            .map(|frame| ns_ms(frame.total_ns()))
            .collect();
        let last = samples.last().copied().unwrap_or(0.0);
        let mut sorted = samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let _ = writeln!(
            out,
            "{name:<14} {:>8} {:>8.3} {:>8.3} {:>8.3} {last:>8.3}",
            samples.len(),
            pct(&sorted, 50),
            pct(&sorted, 95),
            sorted.last().copied().unwrap_or(0.0),
        );
        write_cause_phases(out, frames, name);
    }
}

fn write_cause_phases(out: &mut String, frames: &[Frame], cause: Cause) {
    let picked: Vec<&Frame> = frames.iter().filter(|frame| frame.cause == cause).collect();
    let mut phases: Vec<(Phase, f64)> = Vec::new();
    for frame in &picked {
        for (phase, _) in &frame.phases {
            if !phases.iter().any(|(seen, _)| seen == phase) {
                phases.push((*phase, 0.0));
            }
        }
    }
    for (phase, p50) in &mut phases {
        let mut samples: Vec<f64> = picked
            .iter()
            .map(|frame| value_ms(&frame.phases, *phase))
            .collect();
        samples.sort_by(f64::total_cmp);
        *p50 = pct(&samples, 50);
    }
    phases.retain(|(_, p50)| *p50 > 0.0);
    phases.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut cpu: Vec<f64> = picked
        .iter()
        .map(|frame| ns_ms(frame.total_ns()) - value_ms(&frame.phases, Phase::GpuWait))
        .collect();
    cpu.sort_by(f64::total_cmp);
    let _ = write!(out, "  p50 cpu {:.3} |", pct(&cpu, 50));
    for (phase, p50) in phases.iter().take(CAUSE_PHASES) {
        let _ = write!(out, " {phase} {p50:.3}");
    }
    out.push('\n');
}

fn write_count_rows(out: &mut String, frames: &[Frame]) {
    for name in collect_names(frames, counts_of) {
        let last = frames
            .last()
            .map_or(0, |frame| value_u64(&frame.counts, name));
        let mut samples: Vec<u64> = frames
            .iter()
            .map(|frame| value_u64(&frame.counts, name))
            .collect();
        samples.sort_unstable();
        let _ = writeln!(
            out,
            "{name:<14} {:>8} {:>8} {:>8} {last:>8}",
            pct_u64(&samples, 50),
            pct_u64(&samples, 95),
            samples.last().copied().unwrap_or(0),
        );
    }
}

fn write_total_row(out: &mut String, frames: &[Frame]) {
    let totals: Vec<f64> = frames.iter().map(|f| ns_ms(f.total_ns())).collect();
    let mut sorted = totals.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let _ = writeln!(
        out,
        "{:<14} {:>8.3} {:>8.3} {:>8.3} {:>8.3}",
        "frame",
        pct(&sorted, 50),
        pct(&sorted, 95),
        sorted.last().copied().unwrap_or(0.0),
        totals.last().copied().unwrap_or(0.0)
    );
}

fn collect_names<K: Copy + PartialEq>(frames: &[Frame], pick: Pick<K>) -> Vec<K> {
    let mut names = Vec::new();
    for frame in frames {
        for (name, _) in pick(frame) {
            if !names.contains(name) {
                names.push(*name);
            }
        }
    }
    names
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

pub(super) fn pct(sorted: &[f64], p: usize) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = (sorted.len() - 1) * p / 100;
    sorted.get(i).copied().unwrap_or(0.0)
}

fn pct_u64(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let i = (sorted.len() - 1) * p / 100;
    sorted.get(i).copied().unwrap_or(0)
}
