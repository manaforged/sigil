use super::SigilRunner;
use crate::{
    EquivalenceCheck, EquivalenceCheckSpec, FrameworkRunner, KRAUSEST_TEXT_CELLS, Workload,
    WorkloadParams,
};

fn krausest_scroll() -> Workload {
    Workload {
        workload_id: "virtual-list-krausest-smoke-scroll".into(),
        description: String::new(),
        category: "virtual-list".into(),
        params: WorkloadParams {
            row_count: 200,
            row_height_px: 32.0,
            viewport_width_px: 1280.0,
            viewport_height_px: 1280.0,
            row_shape: "krausest-4-cell".into(),
            scroll_pattern: "linear-one-row-per-frame".into(),
            iterations: 4,
            warmup: 1,
            surface: "layout".into(),
            reuse: true,
            cells_per_row: 4,
        },
        equivalence_checks: vec![EquivalenceCheckSpec {
            name: "text_did_work".into(),
            description: String::new(),
        }],
    }
}

#[test]
fn scroll_measures_new_row_text() {
    let result = SigilRunner::new().run(&krausest_scroll()).expect("runner");
    let got = result.verify.get("text_measure_last").copied().unwrap_or(0);
    assert!(
        got >= u64::try_from(KRAUSEST_TEXT_CELLS).unwrap_or(0),
        "verify={:?}",
        result.verify
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
