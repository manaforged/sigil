//! Text summary of recorded frames.

use std::fmt::Write as _;

use crate::store::{self, FRAME_BUDGET_NS, Frame, INTERACT_BUDGET_NS};

pub(crate) fn summary() -> String {
    let frames = store::frames();
    if frames.is_empty() {
        return String::from("sigil-profile: no frames");
    }
    let mut out = String::from("sigil-profile  (ms)\n");
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
    write_bottlenecks(&mut out, &frames);
    let over = frames
        .iter()
        .filter(|frame| frame.total_ns() > FRAME_BUDGET_NS)
        .count();
    let _ = write!(out, "{} frames", frames.len());
    if over > 0 {
        let _ = write!(out, ", {over} over 16.67ms");
    }
    out.push('\n');
    out
}

fn phases_of(frame: &Frame) -> &[crate::store::Phase] {
    &frame.phases
}

fn counts_of(frame: &Frame) -> &[crate::store::Phase] {
    &frame.counts
}

fn write_named_rows(
    out: &mut String,
    frames: &[Frame],
    pick: fn(&Frame) -> &[crate::store::Phase],
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
    let mut names: Vec<&'static str> = Vec::new();
    for frame in frames {
        if frame.cause != "idle" && !names.contains(&frame.cause) {
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
    }
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

fn write_bottlenecks(out: &mut String, frames: &[Frame]) {
    let slow: Vec<&Frame> = frames
        .iter()
        .filter(|frame| frame.total_ns() > FRAME_BUDGET_NS)
        .collect();
    if slow.is_empty() {
        return;
    }
    out.push_str("\nbottlenecks (over 16.67ms)\n");
    for frame in slow.iter().take(12) {
        let (hot, hot_ns) = frame.hottest().unwrap_or(("", 0));
        let _ = writeln!(
            out,
            "  #{:<4} {:<10} {:>7.1}ms  {hot}={:.1}  cmds={}",
            frame.index,
            frame.cause,
            ns_ms(frame.total_ns()),
            ns_ms(hot_ns),
            frame.counter("scene.cmds")
        );
    }
    let inp = frames
        .iter()
        .filter(|frame| frame.cause != "idle" && frame.total_ns() > INTERACT_BUDGET_NS)
        .count();
    if inp > 0 {
        let _ = writeln!(out, "  {inp} interactions over 200ms (INP)");
    }
}

fn collect_names(
    frames: &[Frame],
    pick: fn(&Frame) -> &[crate::store::Phase],
) -> Vec<&'static str> {
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

fn value_ms(list: &[crate::store::Phase], name: &str) -> f64 {
    ns_ms(value_u64(list, name))
}

fn value_u64(list: &[crate::store::Phase], name: &str) -> u64 {
    list.iter()
        .find(|(n, _)| *n == name)
        .map_or(0, |(_, ns)| *ns)
}

#[allow(clippy::cast_precision_loss, reason = "ns → ms display only")]
fn ns_ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

fn pct(sorted: &[f64], p: usize) -> f64 {
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
