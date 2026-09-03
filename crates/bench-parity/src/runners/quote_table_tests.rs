use crate::{
    EquivalenceCheck, EquivalenceCheckSpec, FrameworkRunner, SigilRunner, Workload, WorkloadParams,
    layout_void,
};

fn tiny(reuse: bool) -> Workload {
    Workload {
        workload_id: "data-table-smoke".into(),
        description: "anti-void smoke".into(),
        category: "data-table".into(),
        params: WorkloadParams {
            row_count: 200,
            row_height_px: 24.0,
            viewport_width_px: 1248.0,
            viewport_height_px: 912.0,
            row_shape: "gpui-24-field".into(),
            scroll_pattern: "linear".into(),
            iterations: 16,
            warmup: 1,
            surface: "layout".into(),
            reuse,
            cells_per_row: 24,
        },
        equivalence_checks: vec![
            spec("visible_window_indices"),
            spec("column_count"),
            spec("layout_did_work"),
            spec("text_did_work"),
        ],
    }
}

fn spec(name: &str) -> EquivalenceCheckSpec {
    EquivalenceCheckSpec {
        name: name.into(),
        description: String::new(),
    }
}

fn assert_real(reuse: bool) {
    let result = SigilRunner::new().run(&tiny(reuse)).expect("runner");
    assert!(
        !layout_void(&result.verify),
        "void run reuse={reuse} verify={:?}",
        result.verify
    );
    assert!(
        matches!(
            result.equivalence_check_results.get("layout_did_work"),
            Some(EquivalenceCheck::Pass)
        ),
        "layout_did_work {:?}",
        result.equivalence_check_results
    );
    assert!(
        matches!(
            result
                .equivalence_check_results
                .get("visible_window_indices"),
            Some(EquivalenceCheck::Pass)
        ),
        "visible {:?}",
        result.equivalence_check_results
    );
    assert!(
        matches!(
            result.equivalence_check_results.get("column_count"),
            Some(EquivalenceCheck::Pass)
        ),
        "column_count {:?}",
        result.equivalence_check_results
    );
    assert!(
        matches!(
            result.equivalence_check_results.get("text_did_work"),
            Some(EquivalenceCheck::Pass)
        ),
        "text_did_work {:?}",
        result.equivalence_check_results
    );
}

#[test]
fn construct_is_not_a_cache_hit() {
    assert_real(false);
}

#[test]
fn scroll_reuse_is_not_a_cache_hit() {
    assert_real(true);
}

#[test]
fn tick_keeps_identity_and_skips_band_measure() {
    let mut workload = tiny(true);
    workload.params.scroll_pattern = String::from("tick-visible");
    workload.params.surface = String::from("layout_paint");
    workload.equivalence_checks.push(spec("identity_did_hold"));
    let result = SigilRunner::new().run(&workload).expect("runner");
    assert!(
        matches!(
            result.equivalence_check_results.get("identity_did_hold"),
            Some(EquivalenceCheck::Pass)
        ),
        "identity {:?}",
        result.equivalence_check_results
    );
    assert!(
        matches!(
            result.equivalence_check_results.get("text_did_work"),
            Some(EquivalenceCheck::Pass)
        ),
        "text_did_work {:?}",
        result.equivalence_check_results
    );
    assert!(
        matches!(
            result.equivalence_check_results.get("scene_did_work"),
            Some(EquivalenceCheck::Pass)
        ),
        "scene_did_work {:?}",
        result.equivalence_check_results
    );
    assert!(
        matches!(
            result.equivalence_check_results.get("last_done_did_move"),
            Some(EquivalenceCheck::Pass)
        ),
        "last_done {:?}",
        result.equivalence_check_results
    );
}

#[test]
fn remount_paint_emits_row_draws() {
    let mut workload = tiny(true);
    workload.params.surface = String::from("layout_paint");
    let result = SigilRunner::new().run(&workload).expect("runner");
    assert!(
        matches!(
            result.equivalence_check_results.get("scene_did_work"),
            Some(EquivalenceCheck::Pass)
        ),
        "scene_did_work {:?}",
        result.equivalence_check_results
    );
    assert!(
        result.verify.get("scene_cmds_last").copied().unwrap_or(0) >= 6,
        "verify={:?}",
        result.verify
    );
}
