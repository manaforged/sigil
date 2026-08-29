use std::collections::{BTreeSet, HashMap};

use crate::{EquivalenceCheck, RunResult};

pub const CV_CEILING: f64 = 0.25;

#[must_use]
#[allow(clippy::implicit_hasher, reason = "intentional, reviewed trade-off")]
pub fn layout_void(verify: &HashMap<String, u64>) -> bool {
    let samples = verify.get("samples").copied().unwrap_or(0);
    let hits = verify.get("layout_hits").copied().unwrap_or(0);
    samples > 0 && hits >= samples && verify.get("tick").copied().unwrap_or(0) == 0
}

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

#[must_use]
pub fn same_checks(a: &RunResult, b: &RunResult) -> bool {
    names(a) == names(b)
}

fn names(result: &RunResult) -> BTreeSet<String> {
    result.equivalence_check_results.keys().cloned().collect()
}

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

#[must_use]
pub fn same_host(a: &RunResult, b: &RunResult) -> bool {
    a.host.os == b.host.os
        && a.host.cpu == b.host.cpu
        && a.host.rustc_version == b.host.rustc_version
}

#[must_use]
pub fn same_mode(a: &RunResult, b: &RunResult) -> bool {
    match (mode(a), mode(b)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

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

#[must_use]
pub fn same_output(a: &RunResult, b: &RunResult) -> bool {
    matches!((&a.output, &b.output), (Some(left), Some(right)) if left == right)
}

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

#[must_use]
pub fn cv_ok(result: &RunResult) -> bool {
    result.summary.cv <= CV_CEILING
}

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

#[must_use]
pub fn is_construct(result: &RunResult) -> bool {
    result.verify.get("reuse").copied() == Some(0)
        && result.verify.get("tick").copied().unwrap_or(0) == 0
}

#[must_use]
pub fn one_to_one(a: &RunResult, b: &RunResult) -> bool {
    comparable(a, b) && is_construct(a) && is_construct(b)
}

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

fn format_ratio(ratio: f64) -> String {
    if ratio > 0.0 && ratio < 0.01 {
        format!("{ratio:.3}×")
    } else {
        format!("{ratio:.2}×")
    }
}

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
