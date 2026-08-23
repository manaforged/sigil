//! Pair gate for `vs gpui`.
//!
//! A numeric ratio prints only when both sides did the same named work
//! on the same host, passed the same verdicts, and neither run is void
//! or noisy.

use std::collections::{BTreeSet, HashMap};

use crate::{EquivalenceCheck, RunResult};

/// Coefficient of variation above this refuses the pair. Median stays
/// in the table; the cell is `NOISY`.
pub const CV_CEILING: f64 = 0.25;

/// True when every sample was a layout cache hit — the timer did no work.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn layout_void(verify: &HashMap<String, u64>) -> bool {
    let samples = verify.get("samples").copied().unwrap_or(0);
    let hits = verify.get("layout_hits").copied().unwrap_or(0);
    samples > 0 && hits >= samples && verify.get("tick").copied().unwrap_or(0) == 0
}

/// True when this result may sit on one side of a pair.
///
/// An empty check map is not a pass.
#[must_use]
pub fn ratio_ok(result: &RunResult) -> bool {
    if layout_void(&result.verify) {
        return false;
    }
    if result.equivalence_check_results.is_empty() {
        return false;
    }
    let mut any_pass = false;
    for verdict in result.equivalence_check_results.values() {
        match verdict {
            EquivalenceCheck::Fail { .. } => return false,
            EquivalenceCheck::Pass => any_pass = true,
            EquivalenceCheck::NotApplicable => {}
        }
    }
    if !any_pass {
        return false;
    }
    if result.workload_id.starts_with("data-table") {
        let table = passed(result, "column_count")
            && passed(result, "layout_did_work")
            && passed_or_na(result, "text_did_work");
        if result.verify.get("tick").copied().unwrap_or(0) == 1 {
            return table && passed(result, "last_done_did_move");
        }
        return table;
    }
    if result.workload_id.contains("krausest") {
        return passed(result, "book_held")
            && passed(result, "visible_window_indices")
            && passed(result, "text_did_work")
            && passed(result, "layout_did_work");
    }
    if crate::text_construct_id(&result.workload_id) {
        return passed(result, "layout_did_work") && passed(result, "text_did_work");
    }
    true
}

fn passed(result: &RunResult, name: &str) -> bool {
    matches!(
        result.equivalence_check_results.get(name),
        Some(EquivalenceCheck::Pass)
    )
}

fn passed_or_na(result: &RunResult, name: &str) -> bool {
    matches!(
        result.equivalence_check_results.get(name),
        Some(EquivalenceCheck::Pass | EquivalenceCheck::NotApplicable)
    )
}

/// True when two results declared the same check names.
#[must_use]
pub fn same_checks(a: &RunResult, b: &RunResult) -> bool {
    names(a) == names(b)
}

fn names(result: &RunResult) -> BTreeSet<String> {
    result.equivalence_check_results.keys().cloned().collect()
}

/// True when every shared check has the same verdict kind.
///
/// Pass vs N/A is not the same work.
#[must_use]
pub fn same_verdicts(a: &RunResult, b: &RunResult) -> bool {
    if !same_checks(a, b) {
        return false;
    }
    a.equivalence_check_results.iter().all(|(name, left)| {
        b.equivalence_check_results
            .get(name)
            .is_some_and(|right| verdict_eq(left, right))
    })
}

const fn verdict_eq(left: &EquivalenceCheck, right: &EquivalenceCheck) -> bool {
    matches!(
        (left, right),
        (EquivalenceCheck::Pass, EquivalenceCheck::Pass)
            | (
                EquivalenceCheck::NotApplicable,
                EquivalenceCheck::NotApplicable
            )
            | (EquivalenceCheck::Fail { .. }, EquivalenceCheck::Fail { .. })
    )
}

/// True when OS, CPU brand, and rustc match.
#[must_use]
pub fn same_host(a: &RunResult, b: &RunResult) -> bool {
    a.host.os == b.host.os
        && a.host.cpu == b.host.cpu
        && a.host.rustc_version == b.host.rustc_version
}

/// True when both sides stamped the same reuse/tick mode.
///
/// Missing `reuse` is not construct — stale JSON cannot pair.
#[must_use]
pub fn same_mode(a: &RunResult, b: &RunResult) -> bool {
    match (mode(a), mode(b)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

/// True when both results came from the same clean source revision.
#[must_use]
pub fn same_source(a: &RunResult, b: &RunResult) -> bool {
    match (&a.source, &b.source) {
        (Some(left), Some(right)) => {
            !left.dirty
                && !right.dirty
                && left.revision != "unknown"
                && left.revision == right.revision
                && left.harness_version == right.harness_version
        }
        _ => false,
    }
}

/// True when both runners emitted the same canonical last-frame output.
#[must_use]
pub fn same_output(a: &RunResult, b: &RunResult) -> bool {
    matches!((&a.output, &b.output), (Some(left), Some(right)) if left == right)
}

/// True when both sides mounted the same number of rows.
///
/// Missing and zero are the same "no window" (flex / labels). A
/// window on one side only, or 76 vs 38, is not 1:1. Canonical output
/// clips overscan, so this check is the one that refuses extra pages.
#[must_use]
pub fn same_visible(a: &RunResult, b: &RunResult) -> bool {
    match (window_rows(a), window_rows(b)) {
        (None, None) => true,
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn window_rows(result: &RunResult) -> Option<u64> {
    result
        .verify
        .get("visible_last")
        .copied()
        .filter(|rows| *rows > 0)
}

fn mode(result: &RunResult) -> Option<(u64, u64)> {
    let reuse = result.verify.get("reuse").copied()?;
    let tick = result.verify.get("tick").copied().unwrap_or(0);
    Some((reuse, tick))
}

/// True when the run's CV is at or under [`CV_CEILING`].
#[must_use]
pub fn cv_ok(result: &RunResult) -> bool {
    result.summary.cv <= CV_CEILING
}

/// True when the two results may share a numeric `vs gpui` cell.
#[must_use]
pub fn comparable(a: &RunResult, b: &RunResult) -> bool {
    ratio_ok(a)
        && ratio_ok(b)
        && same_verdicts(a, b)
        && same_host(a, b)
        && same_mode(a, b)
        && same_source(a, b)
        && same_visible(a, b)
        && same_output(a, b)
        && cv_ok(a)
        && cv_ok(b)
}

/// True when this run reconstructed the tree every sample (not retain, not tick).
#[must_use]
pub fn is_construct(result: &RunResult) -> bool {
    result.verify.get("reuse").copied() == Some(0)
        && result.verify.get("tick").copied().unwrap_or(0) == 0
}

/// Defendable 1:1: [`comparable`] and both sides constructed.
///
/// GPUI 0.2.2 always reconstructs. A Sigil retain/tick run cannot be 1:1
/// with it. Quote [`vs_gpui`] only when this is true.
#[must_use]
pub fn one_to_one(a: &RunResult, b: &RunResult) -> bool {
    comparable(a, b) && is_construct(a) && is_construct(b)
}

/// Pair cell for `row` against `other` (the other framework on this workload).
///
/// A number only when [`comparable`] holds. `1.00×` on the GPUI row
/// requires the Sigil row to be comparable too.
#[must_use]
pub fn vs_gpui(row: &RunResult, other: Option<&RunResult>) -> String {
    if layout_void(&row.verify) {
        return "VOID".into();
    }
    let Some(other) = other else {
        return "—".into();
    };
    if !cv_ok(row) || !cv_ok(other) {
        return "NOISY".into();
    }
    if !same_host(row, other) {
        return "— host".into();
    }
    if !same_mode(row, other) {
        return "— mode".into();
    }
    if !same_source(row, other) {
        return "— source".into();
    }
    if !same_visible(row, other) {
        return "— visible".into();
    }
    if !same_output(row, other) {
        return "— output".into();
    }
    if !same_verdicts(row, other) || !ratio_ok(row) || !ratio_ok(other) {
        return "— checks".into();
    }
    let baseline = if row.framework == "gpui" {
        row.summary.median_us
    } else {
        other.summary.median_us
    };
    if baseline <= 0.0 {
        return "—".into();
    }
    format_ratio(row.summary.median_us / baseline)
}

/// Format a speedup so a sub-0.01 ratio does not print as `0.00×`.
fn format_ratio(ratio: f64) -> String {
    if ratio > 0.0 && ratio < 0.01 {
        format!("{ratio:.3}×")
    } else {
        format!("{ratio:.2}×")
    }
}

/// True when one side retained the tree and the other reconstructed.
///
/// That is a GPUI 0.2.2 app frame (`Render` + `layout_as_root` +
/// `prepaint`) against a Sigil retained `compute`. It is not 1:1 remount.
#[must_use]
pub fn frame_pair(a: &RunResult, b: &RunResult) -> bool {
    ratio_ok(a)
        && ratio_ok(b)
        && same_host(a, b)
        && same_source(a, b)
        && same_output(a, b)
        && cv_ok(a)
        && cv_ok(b)
        && complementary_mode(a, b)
}

fn complementary_mode(a: &RunResult, b: &RunResult) -> bool {
    match (mode(a), mode(b)) {
        (Some((1, left_tick)), Some((0, right_tick)))
        | (Some((0, left_tick)), Some((1, right_tick))) => left_tick == right_tick,
        _ => false,
    }
}

/// Retained-vs-reconstruct cell. A number only when [`frame_pair`] holds.
#[must_use]
pub fn vs_frame(row: &RunResult, other: Option<&RunResult>) -> String {
    if layout_void(&row.verify) {
        return "VOID".into();
    }
    let Some(other) = other else {
        return "—".into();
    };
    if !cv_ok(row) || !cv_ok(other) {
        return "NOISY".into();
    }
    if !same_host(row, other) {
        return "— host".into();
    }
    if !same_source(row, other) {
        return "— source".into();
    }
    if !same_output(row, other) {
        return "— output".into();
    }
    if !complementary_mode(row, other) {
        return "—".into();
    }
    if !ratio_ok(row) || !ratio_ok(other) {
        return "— checks".into();
    }
    let baseline = if row.framework == "gpui" {
        row.summary.median_us
    } else {
        other.summary.median_us
    };
    if baseline <= 0.0 {
        return "—".into();
    }
    format_ratio(row.summary.median_us / baseline)
}

#[cfg(test)]
#[path = "compare_tests.rs"]
mod tests;
