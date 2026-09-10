use sigil::prelude::*;

use crate::endpoints::MockEndpoint;

const PANE_WIDTH: f32 = 360.0;
const PANE_HEIGHT: f32 = 600.0;
const ROW_HEIGHT: f32 = 28.0;
const VIEWPORT_HEIGHT: f32 = 540.0;
const METHOD_COL_WIDTH: f32 = 56.0;

#[must_use]
pub(super) fn build(
    endpoints: &[MockEndpoint],
    selected_index: Signal<usize>,
) -> impl IntoNode + use<> {
    let rows: Vec<(Method, &'static str)> = endpoints
        .iter()
        .map(|e| (Method(e.method.0), e.path))
        .collect();

    let list = virtual_list(rows, move |index, (method, path)| {
        endpoint_row(index, *method, path)
    })
    .row_height(ROW_HEIGHT)
    .viewport_height(VIEWPORT_HEIGHT)
    .viewport_width(PANE_WIDTH)
    .selection(SelectionMode::Single)
    .select(move |index| selected_index.set(index));

    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(PANE_WIDTH, PANE_HEIGHT)
                .gap(4.0),
        )
        .class(classes!("bg-surface"))
        .child(
            text("Endpoints")
                .layout(LayoutStyle::block().width(PANE_WIDTH))
                .class(classes!("text-on-surface text-base font-bold px-2 py-1")),
        )
        .child(list)
}

fn endpoint_row(index: usize, method: Method, path: &'static str) -> NodeBuilder {
    container()
        .role(Role::ListItem)
        .label(format!("Row {index}: {} {path}", method.0))
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(PANE_WIDTH, ROW_HEIGHT)
                .gap(8.0),
        )
        .class(classes!("px-2"))
        .child(
            text(method.0)
                .layout(LayoutStyle::block().width(METHOD_COL_WIDTH))
                .class(method_class(method.0)),
        )
        .child(
            text(path)
                .layout(LayoutStyle::block().width(PANE_WIDTH - METHOD_COL_WIDTH - 16.0))
                .class(classes!("text-on-surface text-sm")),
        )
}

fn method_class(method: &str) -> &'static str {
    match method {
        "GET" => "text-accent text-sm font-bold",
        "POST" | "PUT" | "DELETE" => "text-on-surface text-sm font-bold",
        _ => "text-on-surface text-sm",
    }
}

use crate::endpoints::Method;
