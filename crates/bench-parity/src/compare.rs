//! Pair gate for `vs gpui`.
//!
//! A numeric ratio prints only when both sides did the same named work
//! on the same host, passed the same verdicts, and neither run is void
//! or noisy.

use std::collections::{BTreeSet, HashMap};

use crate::{EquivalenceCheck, RunResult};

#[cfg(test)]
use crate::{HostInfo, RunSummary};

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
            && passed(result, "text_did_work");
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
        && cv_ok(a)
        && cv_ok(b)
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
mod tests {
    use super::*;

    fn dummy(checks: HashMap<String, EquivalenceCheck>) -> RunResult {
        dummy_named("t", "sigil", checks, 0.05)
    }

    fn dummy_named(
        workload_id: &str,
        framework: &str,
        checks: HashMap<String, EquivalenceCheck>,
        cv: f64,
    ) -> RunResult {
        let mut verify = HashMap::from([
            ("samples".into(), 4),
            ("layout_hits".into(), 0),
            ("reuse".into(), 0),
            ("tick".into(), 0),
        ]);
        if workload_id.contains("remount")
            || workload_id.contains("-paint")
            || workload_id.contains("-scroll")
        {
            verify.insert("reuse".into(), 1);
        }
        if workload_id.contains("tick") {
            verify.insert("tick".into(), 1);
            verify.insert("reuse".into(), 1);
        }
        RunResult {
            framework: framework.into(),
            workload_id: workload_id.into(),
            summary: RunSummary {
                iterations: 4,
                median_us: if framework == "gpui" { 100.0 } else { 70.0 },
                p95_us: 110.0,
                p99_us: 120.0,
                mean_us: 80.0,
                cv,
            },
            samples_ns: vec![70_000],
            equivalence_check_results: checks,
            host: HostInfo {
                os: "Darwin".into(),
                cpu: "Apple M2 Max".into(),
                cpu_cores: 12,
                total_memory_bytes: 1,
                rustc_version: "rustc 1.97.1".into(),
                process_rss_bytes: 0,
            },
            verify,
        }
    }

    fn table_checks(text: EquivalenceCheck) -> HashMap<String, EquivalenceCheck> {
        let mut checks = HashMap::new();
        checks.insert("column_count".into(), EquivalenceCheck::Pass);
        checks.insert("layout_did_work".into(), EquivalenceCheck::Pass);
        checks.insert("text_did_work".into(), text);
        checks.insert("visible_window_indices".into(), EquivalenceCheck::Pass);
        checks
    }

    #[test]
    fn empty_checks_are_not_a_ratio() {
        assert!(!ratio_ok(&dummy(HashMap::new())));
    }

    #[test]
    fn one_pass_is_a_ratio() {
        let mut checks = HashMap::new();
        checks.insert("layout_did_work".into(), EquivalenceCheck::Pass);
        assert!(ratio_ok(&dummy(checks)));
    }

    #[test]
    fn million_without_book_held_is_not_a_ratio() {
        let mut checks = HashMap::new();
        checks.insert("visible_window_indices".into(), EquivalenceCheck::Pass);
        let mut result = dummy(checks);
        result.workload_id = "virtual-list-1m-krausest".into();
        assert!(!ratio_ok(&result));
    }

    #[test]
    fn ten_million_without_book_held_is_not_a_ratio() {
        let mut checks = HashMap::new();
        checks.insert("visible_window_indices".into(), EquivalenceCheck::Pass);
        let mut result = dummy(checks);
        result.workload_id = "virtual-list-10m-krausest".into();
        assert!(!ratio_ok(&result));
    }

    #[test]
    fn million_with_book_held_is_a_ratio() {
        let mut checks = HashMap::new();
        checks.insert("visible_window_indices".into(), EquivalenceCheck::Pass);
        checks.insert("book_held".into(), EquivalenceCheck::Pass);
        checks.insert("text_did_work".into(), EquivalenceCheck::Pass);
        let mut result = dummy(checks);
        result.workload_id = "virtual-list-1m-krausest".into();
        assert!(ratio_ok(&result));
    }

    #[test]
    fn krausest_without_text_work_is_not_a_ratio() {
        let mut checks = HashMap::new();
        checks.insert("visible_window_indices".into(), EquivalenceCheck::Pass);
        checks.insert("book_held".into(), EquivalenceCheck::Pass);
        let mut result = dummy(checks);
        result.workload_id = "virtual-list-1m-krausest".into();
        assert!(!ratio_ok(&result));
    }

    #[test]
    fn tick_without_last_done_move_is_not_a_ratio() {
        let mut result = dummy(table_checks(EquivalenceCheck::Pass));
        result.workload_id = "data-table-10k-tick".into();
        result.verify.insert("tick".into(), 1);
        result.verify.insert("reuse".into(), 1);
        assert!(!ratio_ok(&result));
        result
            .equivalence_check_results
            .insert("last_done_did_move".into(), EquivalenceCheck::Pass);
        assert!(ratio_ok(&result));
    }

    #[test]
    fn data_table_without_column_count_is_not_a_ratio() {
        let mut checks = HashMap::new();
        checks.insert("visible_window_indices".into(), EquivalenceCheck::Pass);
        let mut result = dummy(checks);
        result.workload_id = "data-table-10k".into();
        assert!(!ratio_ok(&result));
    }

    #[test]
    fn data_table_clone_checks_are_a_ratio() {
        let mut result = dummy(table_checks(EquivalenceCheck::NotApplicable));
        result.workload_id = "data-table-10k".into();
        assert!(ratio_ok(&result));
    }

    #[test]
    fn one_fail_blocks_the_ratio() {
        let mut checks = HashMap::new();
        checks.insert("layout_did_work".into(), EquivalenceCheck::Pass);
        checks.insert(
            "scene_did_work".into(),
            EquivalenceCheck::Fail {
                detail: "paint panicked".into(),
            },
        );
        assert!(!ratio_ok(&dummy(checks)));
    }

    #[test]
    fn pass_versus_na_is_not_the_same_work() {
        let sigil = dummy_named(
            "data-table-10k",
            "sigil",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        let gpui = dummy_named(
            "data-table-10k",
            "gpui",
            table_checks(EquivalenceCheck::NotApplicable),
            0.05,
        );
        assert!(same_checks(&sigil, &gpui));
        assert!(!same_verdicts(&sigil, &gpui));
        assert!(!comparable(&sigil, &gpui));
        assert_eq!(vs_gpui(&sigil, Some(&gpui)), "— checks");
        assert_eq!(vs_gpui(&gpui, Some(&sigil)), "— checks");
    }

    #[test]
    fn matching_construct_prints_a_ratio() {
        let sigil = dummy_named(
            "data-table-10k",
            "sigil",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        let gpui = dummy_named(
            "data-table-10k",
            "gpui",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        assert!(comparable(&sigil, &gpui));
        assert_eq!(vs_gpui(&sigil, Some(&gpui)), "0.70×");
        assert_eq!(vs_gpui(&gpui, Some(&sigil)), "1.00×");
    }

    #[test]
    fn remount_does_not_pair_with_reconstruct() {
        let sigil = dummy_named(
            "data-table-10k-remount",
            "sigil",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        let gpui = dummy_named(
            "data-table-10k-remount",
            "gpui",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        // GPUI reconstructs every sample — reuse stays 0.
        let mut gpui = gpui;
        gpui.verify.insert("reuse".into(), 0);
        assert!(!same_mode(&sigil, &gpui));
        assert_eq!(vs_gpui(&sigil, Some(&gpui)), "— mode");
        assert_eq!(vs_gpui(&gpui, Some(&sigil)), "— mode");
    }

    #[test]
    fn missing_reuse_stamp_cannot_pair() {
        let mut sigil = dummy_named(
            "data-table-10k",
            "sigil",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        let gpui = dummy_named(
            "data-table-10k",
            "gpui",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        sigil.verify.remove("reuse");
        assert_eq!(vs_gpui(&sigil, Some(&gpui)), "— mode");
    }

    #[test]
    fn noisy_cv_blocks_the_pair() {
        let sigil = dummy_named(
            "data-table-10k",
            "sigil",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        let gpui = dummy_named(
            "data-table-10k",
            "gpui",
            table_checks(EquivalenceCheck::Pass),
            0.60,
        );
        assert!(!comparable(&sigil, &gpui));
        assert_eq!(vs_gpui(&sigil, Some(&gpui)), "NOISY");
        assert_eq!(vs_gpui(&gpui, Some(&sigil)), "NOISY");
    }

    #[test]
    fn host_mismatch_blocks_the_pair() {
        let sigil = dummy_named(
            "data-table-10k",
            "sigil",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        let mut gpui = dummy_named(
            "data-table-10k",
            "gpui",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        gpui.host.cpu = "other".into();
        assert_eq!(vs_gpui(&sigil, Some(&gpui)), "— host");
    }

    #[test]
    fn gpui_alone_is_not_a_baseline() {
        let gpui = dummy_named(
            "data-table-10k",
            "gpui",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        assert_eq!(vs_gpui(&gpui, None), "—");
    }

    #[test]
    fn remount_prints_a_frame_ratio_not_vs_gpui() {
        let sigil = dummy_named(
            "data-table-10k-remount",
            "sigil",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        let mut gpui = dummy_named(
            "data-table-10k-remount",
            "gpui",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        gpui.verify.insert("reuse".into(), 0);
        assert_eq!(vs_gpui(&sigil, Some(&gpui)), "— mode");
        assert_eq!(vs_frame(&sigil, Some(&gpui)), "0.70×");
        assert_eq!(vs_frame(&gpui, Some(&sigil)), "1.00×");
    }

    #[test]
    fn scroll_prints_a_frame_ratio_not_vs_gpui() {
        let mut checks = HashMap::new();
        checks.insert("visible_window_indices".into(), EquivalenceCheck::Pass);
        checks.insert("book_held".into(), EquivalenceCheck::Pass);
        checks.insert("text_did_work".into(), EquivalenceCheck::Pass);
        let sigil = dummy_named(
            "virtual-list-1m-krausest-scroll",
            "sigil",
            checks.clone(),
            0.05,
        );
        let mut gpui = dummy_named("virtual-list-1m-krausest-scroll", "gpui", checks, 0.05);
        gpui.verify.insert("reuse".into(), 0);
        assert_eq!(vs_gpui(&sigil, Some(&gpui)), "— mode");
        assert_eq!(vs_frame(&sigil, Some(&gpui)), "0.70×");
        assert_eq!(vs_frame(&gpui, Some(&sigil)), "1.00×");
    }

    #[test]
    fn construct_has_no_frame_ratio() {
        let sigil = dummy_named(
            "data-table-10k",
            "sigil",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        let gpui = dummy_named(
            "data-table-10k",
            "gpui",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        assert_eq!(vs_gpui(&sigil, Some(&gpui)), "0.70×");
        assert_eq!(vs_frame(&sigil, Some(&gpui)), "—");
    }

    #[test]
    fn tiny_frame_ratio_keeps_three_decimals() {
        let mut sigil = dummy_named(
            "data-table-10k-tick",
            "sigil",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        sigil
            .equivalence_check_results
            .insert("last_done_did_move".into(), EquivalenceCheck::Pass);
        sigil.summary.median_us = 7.0;
        let mut gpui = dummy_named(
            "data-table-10k-tick",
            "gpui",
            table_checks(EquivalenceCheck::Pass),
            0.05,
        );
        gpui.equivalence_check_results
            .insert("last_done_did_move".into(), EquivalenceCheck::Pass);
        gpui.verify.insert("reuse".into(), 0);
        gpui.summary.median_us = 3500.0;
        assert_eq!(vs_frame(&sigil, Some(&gpui)), "0.002×");
    }
}
