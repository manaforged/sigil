use std::collections::BTreeMap;

use crate::{OutputProof, Workload};

pub const NESTED_LEAF: &str = "leaf";

#[must_use]
pub fn wrap_paragraph(index: usize) -> String {
    format!("Paragraph {index} wraps across several lines of payload text for layout.")
}

#[must_use]
pub fn form_key(index: usize) -> String {
    format!("Name {index}")
}

#[must_use]
pub fn form_value(index: usize) -> String {
    format!("Value {index}")
}

#[must_use]
pub fn overlay_label(index: usize) -> String {
    format!("card {index}")
}

#[must_use]
pub fn text_construct(category: &str) -> bool {
    matches!(
        category,
        "text-labels" | "wrap-text" | "kv-form" | "nested-flex" | "overlay-stack"
    )
}

#[must_use]
pub fn text_construct_id(workload_id: &str) -> bool {
    workload_id.starts_with("text-labels")
        || workload_id.starts_with("wrap-text")
        || workload_id.starts_with("kv-form")
        || workload_id.starts_with("nested-flex")
        || workload_id.starts_with("overlay-stack")
}

#[must_use]
pub fn expected_texts(workload: &Workload) -> Vec<String> {
    let n = workload.params.row_count;
    match workload.category.as_str() {
        "text-labels" => (0..n)
            .map(|index| format!("row {index} label payload"))
            .collect(),
        "wrap-text" => (0..n).map(wrap_paragraph).collect(),
        "kv-form" => (0..n)
            .flat_map(|index| [form_key(index), form_value(index)])
            .collect(),
        "nested-flex" => vec![NESTED_LEAF.to_string(); n],
        "overlay-stack" => (0..n).map(overlay_label).collect(),
        _ => Vec::new(),
    }
}

#[must_use]
pub fn construct_proof(
    workload: &Workload,
    root_size: [i64; 2],
    texts: Vec<String>,
) -> OutputProof {
    let p = &workload.params;
    let mut values = BTreeMap::new();
    values.insert("rows".into(), u64::try_from(p.row_count).unwrap_or(0));
    if matches!(workload.category.as_str(), "flex-tree" | "kv-form") {
        values.insert(
            "cells_per_row".into(),
            u64::try_from(p.cells()).unwrap_or(0),
        );
    }
    OutputProof {
        schema: if texts.is_empty() {
            "flex-shape-v1".into()
        } else {
            "text-stack-v1".into()
        },
        root_size_milli_px: root_size,
        values,
        text_rows: if texts.is_empty() {
            Vec::new()
        } else {
            vec![texts]
        },
    }
}

#[cfg(test)]
#[path = "construct_tests.rs"]
mod tests;
