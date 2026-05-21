//! Right pane: tabs (Request / Response / Headers) routed off the
//! `selected_index` signal.
//!
//! All three tab bodies are built once at `app_tree`-build time; each
//! body consults `selected_index` reactively via a `derived` string
//! signal so switching endpoints re-renders the visible tab without
//! rebuilding the kernel tree.

use sigil::prelude::*;
use sigil::reactive::{Signal, derived, signal};

use crate::endpoints::{MockEndpoint, header_block, response_summary};

const PANE_WIDTH: f32 = 656.0;
const PANE_HEIGHT: f32 = 600.0;
const PANEL_HEIGHT: f32 = PANE_HEIGHT - 40.0;
const BODY_WIDTH: f32 = PANE_WIDTH - 16.0;

/// Build the right pane.
#[must_use]
pub(super) fn build(
    endpoints: Vec<MockEndpoint>,
    selected_index: Signal<usize>,
) -> impl IntoNode + use<> {
    let active_tab = signal(0_usize);

    let request_panel = request_tab(endpoints.clone(), selected_index);
    let response_panel = response_tab(endpoints.clone(), selected_index);
    let headers_panel = headers_tab(endpoints, selected_index);

    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(PANE_WIDTH, PANE_HEIGHT)
                .gap(4.0),
        )
        .class("bg-surface")
        .child(
            tabs(
                active_tab,
                vec!["Request".into(), "Response".into(), "Headers".into()],
            )
            .label("Endpoint detail")
            .content_for(0, request_panel)
            .content_for(1, response_panel)
            .content_for(2, headers_panel),
        )
}

fn request_tab(
    endpoints: Vec<MockEndpoint>,
    selected_index: Signal<usize>,
) -> impl IntoNode + use<> {
    let body = endpoint_derived(endpoints, selected_index, |e| {
        let mut out = String::with_capacity(e.summary.len() + e.request_body.len() + 32);
        out.push_str(e.method.0);
        out.push(' ');
        out.push_str(e.path);
        out.push_str("\n\n");
        out.push_str(e.summary);
        out.push_str("\n\n");
        out.push_str(e.request_body);
        out
    });
    panel_body(
        text(body)
            .layout(LayoutStyle::block().width(BODY_WIDTH))
            .class("font-mono text-sm"),
    )
}

fn response_tab(
    endpoints: Vec<MockEndpoint>,
    selected_index: Signal<usize>,
) -> impl IntoNode + use<> {
    let body = endpoint_derived(endpoints, selected_index, |e| {
        let mut out = response_summary(e);
        out.push_str(e.response_body);
        out
    });
    panel_body(
        text(body)
            .layout(LayoutStyle::block().width(BODY_WIDTH))
            .class("font-mono text-sm"),
    )
}

fn headers_tab(
    endpoints: Vec<MockEndpoint>,
    selected_index: Signal<usize>,
) -> impl IntoNode + use<> {
    let body = endpoint_derived(endpoints, selected_index, header_block);
    panel_body(
        text(body)
            .layout(LayoutStyle::block().width(BODY_WIDTH))
            .class("font-mono text-sm"),
    )
}

/// Wrap the per-endpoint string projection in a `derived` signal that
/// clamps the index to the catalog length on every read.
fn endpoint_derived(
    endpoints: Vec<MockEndpoint>,
    selected_index: Signal<usize>,
    project: fn(&MockEndpoint) -> String,
) -> Signal<String> {
    let count = endpoints.len();
    derived(move || {
        let idx = selected_index.get().min(count.saturating_sub(1));
        endpoints.get(idx).map(project).unwrap_or_default()
    })
}

fn panel_body(body: impl IntoNode + 'static) -> impl IntoNode {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(PANE_WIDTH, PANEL_HEIGHT)
                .gap(4.0),
        )
        .class("p-2 bg-surface")
        .child(body)
}
