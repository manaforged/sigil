//! Non-list 1:1 construct trees: wrap, nested flex, kv form, overlay.

use sigil_kernel::{NodeBuilder, container, text};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle, Position};
use sigil_reactive::Runtime;

use super::sigil_runner::sample_build;
use crate::{
    NESTED_LEAF, Workload, form_key, form_value, overlay_label, text_construct, wrap_paragraph,
};

pub(super) fn run(workload: &Workload) -> Result<crate::RunResult, String> {
    crate::runners::sigil_runner::validate(workload.params.row_count, workload.params.iterations)?;
    let real_text = text_construct(&workload.category);
    Ok(Runtime::scope("sigil-bench-parity/construct", || {
        sample_build(workload, real_text, || tree(workload))
    }))
}

fn tree(workload: &Workload) -> NodeBuilder {
    match workload.category.as_str() {
        "wrap-text" => wrap_text(workload),
        "kv-form" => kv_form(workload),
        "nested-flex" => nested_flex(workload),
        _ => overlay_stack(workload),
    }
}

fn wrap_text(workload: &Workload) -> NodeBuilder {
    let p = &workload.params;
    let mut col = container()
        .layout(LayoutStyle::flex_column().fixed_size(p.viewport_width_px, p.viewport_height_px));
    for index in 0..p.row_count {
        col = col.child(
            container()
                .layout(LayoutStyle::block().width(p.viewport_width_px))
                .child(text(wrap_paragraph(index)).wrap()),
        );
    }
    col
}

fn kv_form(workload: &Workload) -> NodeBuilder {
    let p = &workload.params;
    let key_w = 240.0;
    let val_w = (p.viewport_width_px - key_w).max(p.row_height_px);
    let mut col = container()
        .layout(LayoutStyle::flex_column().fixed_size(p.viewport_width_px, p.viewport_height_px));
    for index in 0..p.row_count {
        col = col.child(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(p.viewport_width_px, p.row_height_px))
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(key_w, p.row_height_px))
                        .child(text(form_key(index))),
                )
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(val_w, p.row_height_px))
                        .child(text(form_value(index))),
                ),
        );
    }
    col
}

fn nested_flex(workload: &Workload) -> NodeBuilder {
    let p = &workload.params;
    let depth = p.cells().max(1);
    let mut col = container()
        .layout(LayoutStyle::flex_column().fixed_size(p.viewport_width_px, p.viewport_height_px));
    for _ in 0..p.row_count {
        col = col.child(nested_spine(depth, p.viewport_width_px));
    }
    col
}

fn nested_spine(depth: usize, width: f32) -> NodeBuilder {
    let mut node = container()
        .layout(LayoutStyle::flex_column().width(width))
        .child(text(NESTED_LEAF));
    for _ in 1..depth {
        node = container()
            .layout(LayoutStyle::flex_column().width(width))
            .child(node);
    }
    node
}

fn overlay_stack(workload: &Workload) -> NodeBuilder {
    let p = &workload.params;
    let mut host = container().layout(
        LayoutStyle::block()
            .fixed_size(p.viewport_width_px, p.viewport_height_px)
            .position(Position::Relative),
    );
    for index in 0..p.row_count {
        #[allow(clippy::cast_precision_loss)]
        let inset = index as f32 * 4.0;
        host = host.child(
            container()
                .layout(
                    LayoutStyle::block()
                        .fixed_size(120.0, p.row_height_px)
                        .position(Position::Absolute)
                        .inset_top(inset)
                        .inset_left(inset),
                )
                .child(text(overlay_label(index))),
        );
    }
    host
}
