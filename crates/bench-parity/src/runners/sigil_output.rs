use std::collections::HashMap;

use sigil_hal::{Rect, Size};
use sigil_kernel::{Cx, NodeBuilder, NodeId};
use sigil_layout::LayoutResult;

use crate::{EquivalenceCheck, OutputProof, Workload, milli_px};

use super::new_engine;

pub(super) fn probe_build(
    workload: &Workload,
    real_text: bool,
    build: &mut impl FnMut() -> NodeBuilder,
) -> OutputProof {
    let p = &workload.params;
    let mut cx = Cx::new_root();
    let mut engine = new_engine(real_text);
    let root = cx.add(build());
    let _ = engine.compute(
        &mut cx,
        root,
        Size::fixed(p.viewport_width_px, p.viewport_height_px),
    );
    let texts = if real_text {
        text_values(&cx, root)
    } else {
        Vec::new()
    };
    crate::construct_proof(workload, root_size(&cx, root), texts)
}

pub(in crate::runners) fn list_output(cx: &Cx, root: NodeId, first: usize) -> OutputProof {
    let text_rows = visible_text_rows(cx, root);
    let actual_first = text_rows
        .first()
        .and_then(|row| row.first())
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(first);
    let mut values = std::collections::BTreeMap::new();
    values.insert("first_row".into(), u64::try_from(actual_first).unwrap_or(0));
    values.insert(
        "visible_rows".into(),
        u64::try_from(text_rows.len()).unwrap_or(0),
    );
    OutputProof {
        schema: "visible-rows-v1".into(),
        root_size_milli_px: root_size(cx, root),
        values,
        text_rows,
    }
}

fn visible_text_rows(cx: &Cx, root: NodeId) -> Vec<Vec<String>> {
    let Some(clip) = cx
        .get(root)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .map(LayoutResult::rect)
    else {
        return Vec::new();
    };
    let Some(host) = cx.first_child(root) else {
        return Vec::new();
    };
    let Some(content) = cx.first_child(host) else {
        return Vec::new();
    };
    let Some(window) = cx.first_child(content) else {
        return Vec::new();
    };
    (0..cx.child_count(window))
        .filter_map(|index| cx.nth_child(window, index))
        .filter(|row| row_intersects(cx, *row, clip))
        .map(|row| text_values(cx, row))
        .collect()
}

fn row_intersects(cx: &Cx, row: NodeId, clip: Rect) -> bool {
    cx.get(row)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .is_some_and(|layout| {
            let rect = layout.rect();
            rect.origin.y < clip.origin.y + clip.height
                && rect.origin.y + rect.height > clip.origin.y
        })
}

fn text_values(cx: &Cx, root: NodeId) -> Vec<String> {
    let mut out = Vec::new();
    collect_text(cx, root, &mut out);
    out
}

fn collect_text(cx: &Cx, id: NodeId, out: &mut Vec<String>) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if let sigil_kernel::NodeType::Text { content, .. } = &node.kind {
        out.push(content.resolve());
    }
    for child in node.children() {
        collect_text(cx, *child, out);
    }
}

fn root_size(cx: &Cx, root: NodeId) -> [i64; 2] {
    cx.get(root)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .map_or([0, 0], |layout| {
            let rect = layout.rect();
            [milli_px(rect.width), milli_px(rect.height)]
        })
}

pub(super) fn build_checks(
    verify: &HashMap<String, u64>,
    real_text: bool,
    measures: u64,
) -> HashMap<String, EquivalenceCheck> {
    let mut extra = HashMap::new();
    extra.insert(
        "layout_did_work".into(),
        if crate::layout_void(verify) {
            EquivalenceCheck::Fail {
                detail: "layout_hits == samples".into(),
            }
        } else {
            EquivalenceCheck::Pass
        },
    );
    if real_text {
        extra.insert(
            "text_did_work".into(),
            if measures > 0 {
                EquivalenceCheck::Pass
            } else {
                EquivalenceCheck::Fail {
                    detail: "text.measure == 0 on every sample".into(),
                }
            },
        );
    }
    extra
}
