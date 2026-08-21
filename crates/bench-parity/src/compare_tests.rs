use super::*;
use crate::{HostInfo, OutputProof, RunSummary, SourceInfo};

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
        output: Some(OutputProof {
            schema: "test-v1".into(),
            root_size_milli_px: [100_000, 100_000],
            values: std::collections::BTreeMap::default(),
            text_rows: vec![vec!["same".into()]],
        }),
        source: Some(SourceInfo {
            revision: "abc123".into(),
            dirty: false,
            harness_version: "0.1.0".into(),
        }),
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
fn missing_output_blocks_the_pair() {
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
    gpui.output = None;
    assert!(!comparable(&sigil, &gpui));
    assert_eq!(vs_gpui(&sigil, Some(&gpui)), "— output");
}

#[test]
fn different_output_blocks_the_pair() {
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
    if let Some(output) = gpui.output.as_mut() {
        output.text_rows = vec![vec!["different".into()]];
    }
    assert!(!comparable(&sigil, &gpui));
    assert_eq!(vs_gpui(&sigil, Some(&gpui)), "— output");
}

#[test]
fn dirty_or_different_source_blocks_the_pair() {
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
    if let Some(source) = gpui.source.as_mut() {
        source.dirty = true;
    }
    assert_eq!(vs_gpui(&sigil, Some(&gpui)), "— source");
    if let Some(source) = gpui.source.as_mut() {
        source.dirty = false;
        source.revision = "def456".into();
    }
    assert_eq!(vs_gpui(&sigil, Some(&gpui)), "— source");
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
