use super::{
    NESTED_LEAF, expected_texts, form_key, form_value, overlay_label, text_construct,
    text_construct_id, wrap_paragraph,
};
use crate::{Workload, WorkloadParams};

fn workload(category: &str, id: &str, n: usize) -> Workload {
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
            iterations: 1,
            warmup: 0,
            surface: String::from("layout"),
            reuse: false,
            cells_per_row: 2,
        },
        equivalence_checks: Vec::new(),
    }
}

#[test]
fn wrap_kv_nested_overlay_are_text_constructs() {
    assert!(text_construct("wrap-text"));
    assert!(text_construct("kv-form"));
    assert!(text_construct("nested-flex"));
    assert!(text_construct("overlay-stack"));
    assert!(text_construct_id("wrap-text-200"));
    assert!(text_construct_id("kv-form-500"));
    assert!(!text_construct("flex-tree"));
    assert!(!text_construct_id("virtual-list-100k-krausest"));
    assert!(!text_construct_id("data-table-10k"));
}

#[test]
fn expected_texts_match_row_count_and_order() {
    let wrap = expected_texts(&workload("wrap-text", "wrap-text-200", 3));
    assert_eq!(
        wrap,
        vec![wrap_paragraph(0), wrap_paragraph(1), wrap_paragraph(2)]
    );

    let form = expected_texts(&workload("kv-form", "kv-form-500", 2));
    assert_eq!(
        form,
        vec![form_key(0), form_value(0), form_key(1), form_value(1)]
    );

    let nested = expected_texts(&workload("nested-flex", "nested-flex-64", 2));
    assert_eq!(
        nested,
        vec![NESTED_LEAF.to_string(), NESTED_LEAF.to_string()]
    );

    let overlay = expected_texts(&workload("overlay-stack", "overlay-stack-100", 2));
    assert_eq!(overlay, vec![overlay_label(0), overlay_label(1)]);
}
