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
            iterations: 16,
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

fn construct_smoke(category: &str, id: &str, n: usize, cells: usize) -> Workload {
    Workload {
        workload_id: id.into(),
        description: String::new(),
        category: category.into(),
        params: WorkloadParams {
            row_count: n,
            row_height_px: 24.0,
            viewport_width_px: 640.0,
            viewport_height_px: 480.0,
            row_shape: String::new(),
            scroll_pattern: String::from("none"),
            iterations: 2,
            warmup: 1,
            surface: String::from("layout"),
            reuse: false,
            cells_per_row: cells,
        },
        equivalence_checks: vec![
            EquivalenceCheckSpec {
                name: "layout_did_work".into(),
                description: String::new(),
            },
            EquivalenceCheckSpec {
                name: "text_did_work".into(),
                description: String::new(),
            },
        ],
    }
}

fn assert_text_construct(category: &str, id: &str, n: usize, cells: usize, want_texts: usize) {
    let result = SigilRunner::new()
        .run(&construct_smoke(category, id, n, cells))
        .expect("runner");
    assert!(
        matches!(
            result.equivalence_check_results.get("layout_did_work"),
            Some(EquivalenceCheck::Pass)
        ),
        "layout {:?}",
        result.equivalence_check_results
    );
    assert!(
        matches!(
            result.equivalence_check_results.get("text_did_work"),
            Some(EquivalenceCheck::Pass)
        ),
        "text {:?}",
        result.equivalence_check_results
    );
    let got = result.verify.get("text_measure_last").copied().unwrap_or(0);
    assert!(
        got >= u64::try_from(want_texts).unwrap_or(0),
        "text.measure {got} < {want_texts} verify={:?}",
        result.verify
    );
    let texts = result
        .output
        .as_ref()
        .map_or(0, |output| output.text_rows.first().map_or(0, Vec::len));
    assert_eq!(texts, want_texts, "scraped {:?}", result.output);
}

#[test]
fn wrap_text_shapes_paragraphs() {
    assert_text_construct("wrap-text", "wrap-text-smoke", 8, 1, 8);
}

#[test]
fn kv_form_shapes_two_cells_per_row() {
    assert_text_construct("kv-form", "kv-form-smoke", 6, 2, 12);
}

#[test]
fn nested_flex_shapes_one_leaf_per_spine() {
    assert_text_construct("nested-flex", "nested-flex-smoke", 4, 3, 4);
}

#[test]
fn overlay_stack_shapes_card_labels() {
    assert_text_construct("overlay-stack", "overlay-stack-smoke", 5, 1, 5);
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
