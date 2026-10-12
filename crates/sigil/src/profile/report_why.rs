use std::cmp::Ordering;
use std::fmt::Write as _;

use crate::profile::attr::Attribution;
use crate::profile::key::{Cause, Counter, Hotspot};
use crate::profile::report::{distinct, ns_ms, spread};
use crate::profile::store::{Frame, INTERACT_BUDGET_NS};

const TOP_PASSES: usize = 12;
const TOP_SUBJECTS: usize = 8;
const TOP_SLOW: usize = 12;

pub(super) fn write_gpu_passes(out: &mut String, frames: &[Frame]) {
    let mut labels: Vec<(&'static str, Vec<f64>)> = Vec::new();
    for frame in frames {
        for &(label, ns) in &frame.gpu_passes {
            match labels.iter_mut().find(|(l, _)| *l == label) {
                Some((_, samples)) => samples.push(ns_ms(ns)),
                None => labels.push((label, vec![ns_ms(ns)])),
            }
        }
    }
    if labels.is_empty() {
        return;
    }
    let mut rows: Vec<(&'static str, [f64; 4])> = labels
        .into_iter()
        .map(|(label, samples)| (label, spread(samples)))
        .collect();
    rows.sort_by(|a, b| b.1[0].partial_cmp(&a.1[0]).unwrap_or(Ordering::Equal));
    out.push_str("\ngpu passes          p50      p95      max\n");
    for (label, [p50, p95, max, _]) in rows.into_iter().take(TOP_PASSES) {
        let _ = writeln!(out, "{label:<16} {p50:>8.3} {p95:>8.3} {max:>8.3}");
    }
}

struct Agg<'a> {
    hotspot: Hotspot,
    subject: &'a str,
    total: u64,
    frames: u64,
}

fn aggregate(frames: &[Frame]) -> Vec<Agg<'_>> {
    let mut aggs: Vec<Agg<'_>> = Vec::new();
    for frame in frames {
        for Attribution {
            hotspot,
            subject,
            count,
        } in &frame.hot
        {
            match aggs
                .iter_mut()
                .find(|a| a.hotspot == *hotspot && a.subject == subject)
            {
                Some(agg) => {
                    agg.total = agg.total.saturating_add(*count);
                    agg.frames += 1;
                }
                None => aggs.push(Agg {
                    hotspot: *hotspot,
                    subject,
                    total: *count,
                    frames: 1,
                }),
            }
        }
    }
    aggs.sort_by_key(|agg| std::cmp::Reverse(agg.total));
    aggs
}

pub(super) fn write_why(out: &mut String, frames: &[Frame]) {
    let aggs = aggregate(frames);
    if aggs.is_empty() {
        return;
    }
    let hotspots = distinct(aggs.iter().map(|agg| agg.hotspot));
    out.push_str("\nwhy\n");
    for hotspot in hotspots {
        for agg in aggs
            .iter()
            .filter(|a| a.hotspot == hotspot)
            .take(TOP_SUBJECTS)
        {
            let _ = writeln!(
                out,
                "  {:<20} {} \u{d7}{}/frame ({} frames)",
                hotspot,
                agg.subject,
                agg.total / agg.frames.max(1),
                agg.frames
            );
        }
    }
}

pub(super) fn write_bottlenecks(out: &mut String, frames: &[Frame], budget: u64) {
    let mut slow: Vec<&Frame> = frames
        .iter()
        .filter(|frame| frame.total_ns() > budget)
        .collect();
    if !slow.is_empty() {
        slow.sort_by_key(|frame| std::cmp::Reverse(frame.total_ns()));
        let _ = writeln!(out, "\nbottlenecks (over {:.2}ms)", ns_ms(budget));
        for frame in slow.iter().take(TOP_SLOW) {
            let (hot, hot_ns) = frame
                .hottest()
                .map_or(("", 0), |(phase, ns)| (phase.name(), ns));
            let _ = write!(
                out,
                "  #{:<4} {:<10} {:>7.1}ms  {hot}={:.1}  cmds={}",
                frame.index,
                frame.cause,
                ns_ms(frame.total_ns()),
                ns_ms(hot_ns),
                frame.counter(Counter::SceneCmds)
            );
            if let Some(top) = frame.hot.iter().max_by_key(|a| a.count) {
                let _ = write!(out, "  {} {} \u{d7}{}", top.hotspot, top.subject, top.count);
            }
            out.push('\n');
        }
    }
    let inp = frames
        .iter()
        .filter(|frame| frame.cause != Cause::Idle && frame.total_ns() > INTERACT_BUDGET_NS)
        .count();
    if inp > 0 {
        let _ = writeln!(
            out,
            "  {inp} interactions over 200ms (Chrome INP; native bar is {:.2}ms)",
            ns_ms(budget)
        );
    }
}
