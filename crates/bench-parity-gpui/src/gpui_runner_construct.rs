use bench_parity::{NESTED_LEAF, Workload, form_key, form_value, overlay_label, wrap_paragraph};
use gpui::{AnyElement, IntoElement, ParentElement, Styled, div, px};

use super::build::{sample_build, validate};
use super::timing::new_host;

pub(super) fn run(workload: &Workload) -> Result<bench_parity::RunResult, String> {
    validate(&workload.params)?;
    let mut host = new_host();
    sample_build(workload, &mut host, || tree(workload))
}

fn tree(workload: &Workload) -> AnyElement {
    match workload.category.as_str() {
        "wrap-text" => wrap_text(workload),
        "kv-form" => kv_form(workload),
        "nested-flex" => nested_flex(workload),
        _ => overlay_stack(workload),
    }
}

fn wrap_text(workload: &Workload) -> AnyElement {
    let p = &workload.params;
    let mut col = div()
        .flex()
        .flex_col()
        .w(px(p.viewport_width_px))
        .h(px(p.viewport_height_px));
    for index in 0..p.row_count {
        col = col.child(
            div()
                .w(px(p.viewport_width_px))
                .whitespace_normal()
                .child(wrap_paragraph(index)),
        );
    }
    col.into_any_element()
}

fn kv_form(workload: &Workload) -> AnyElement {
    let p = &workload.params;
    let key_w = 240.0;
    let val_w = (p.viewport_width_px - key_w).max(p.row_height_px);
    let mut col = div()
        .flex()
        .flex_col()
        .w(px(p.viewport_width_px))
        .h(px(p.viewport_height_px));
    for index in 0..p.row_count {
        col = col.child(
            div()
                .flex()
                .flex_row()
                .w(px(p.viewport_width_px))
                .h(px(p.row_height_px))
                .child(
                    div()
                        .w(px(key_w))
                        .h(px(p.row_height_px))
                        .child(form_key(index)),
                )
                .child(
                    div()
                        .w(px(val_w))
                        .h(px(p.row_height_px))
                        .child(form_value(index)),
                ),
        );
    }
    col.into_any_element()
}

fn nested_flex(workload: &Workload) -> AnyElement {
    let p = &workload.params;
    let depth = p.cells().max(1);
    let mut col = div()
        .flex()
        .flex_col()
        .w(px(p.viewport_width_px))
        .h(px(p.viewport_height_px));
    for _ in 0..p.row_count {
        col = col.child(nested_spine(depth, p.viewport_width_px));
    }
    col.into_any_element()
}

fn nested_spine(depth: usize, width: f32) -> AnyElement {
    let mut node = div()
        .flex()
        .flex_col()
        .w(px(width))
        .child(NESTED_LEAF)
        .into_any_element();
    for _ in 1..depth {
        node = div()
            .flex()
            .flex_col()
            .w(px(width))
            .child(node)
            .into_any_element();
    }
    node
}

fn overlay_stack(workload: &Workload) -> AnyElement {
    let p = &workload.params;
    let mut host = div()
        .relative()
        .w(px(p.viewport_width_px))
        .h(px(p.viewport_height_px));
    for index in 0..p.row_count {
        #[allow(
            clippy::cast_precision_loss,
            reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
        )]
        let inset = px(index as f32 * 4.0);
        host = host.child(
            div()
                .absolute()
                .top(inset)
                .left(inset)
                .w(px(120.0))
                .h(px(p.row_height_px))
                .child(overlay_label(index)),
        );
    }
    host.into_any_element()
}
