//! Right pane: tabs (Preview / Metadata / Hex) routed off the
//! `selected_index` signal.
//!
//! All three tab bodies are built once at `app_tree`-build time; each
//! body consults `selected_index` reactively via a `derived` string
//! signal so switching files re-renders the visible tab without
//! rebuilding the kernel tree.

use std::fmt::Write as _;

use sigil::prelude::*;
use sigil::reactive::{Signal, derived, signal};
use sigil::widgets::tabs;

use crate::files::{MockFile, hex_dump};

const RIGHT_WIDTH: f32 = 656.0;
const RIGHT_HEIGHT: f32 = 600.0;

/// Build the right pane.
#[must_use]
pub(super) fn build(files: Vec<MockFile>, selected_index: Signal<usize>) -> impl IntoNode + use<> {
    let active_tab = signal(0_usize);

    let preview_panel = preview_tab(files.clone(), selected_index);
    let metadata_panel = metadata_tab(files.clone(), selected_index);
    let hex_panel = hex_tab(files, selected_index);

    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(RIGHT_WIDTH, RIGHT_HEIGHT)
                .gap(4.0),
        )
        .class("bg-surface")
        .child(
            tabs(
                active_tab,
                vec!["Preview".into(), "Metadata".into(), "Hex".into()],
            )
            .label("File detail")
            .content_for(0, preview_panel)
            .content_for(1, metadata_panel)
            .content_for(2, hex_panel),
        )
}

fn preview_tab(files: Vec<MockFile>, selected_index: Signal<usize>) -> impl IntoNode + use<> {
    let count = files.len();
    let body = derived(move || {
        let idx = selected_index.get().min(count.saturating_sub(1));
        files
            .get(idx)
            .map(|file| file.preview.to_owned())
            .unwrap_or_default()
    });
    panel_body(text(body).layout(LayoutStyle::block().width(RIGHT_WIDTH - 16.0)))
}

fn metadata_tab(files: Vec<MockFile>, selected_index: Signal<usize>) -> impl IntoNode + use<> {
    let count = files.len();
    let body = derived(move || {
        let idx = selected_index.get().min(count.saturating_sub(1));
        files.get(idx).map(format_metadata).unwrap_or_default()
    });
    panel_body(text(body).layout(LayoutStyle::block().width(RIGHT_WIDTH - 16.0)))
}

fn hex_tab(files: Vec<MockFile>, selected_index: Signal<usize>) -> impl IntoNode + use<> {
    let count = files.len();
    let body = derived(move || {
        let idx = selected_index.get().min(count.saturating_sub(1));
        files
            .get(idx)
            .map(|file| hex_dump(idx, file.size))
            .unwrap_or_default()
    });
    panel_body(
        text(body)
            .layout(LayoutStyle::block().width(RIGHT_WIDTH - 16.0))
            .class("font-mono text-sm"),
    )
}

fn panel_body(body: impl IntoNode + 'static) -> impl IntoNode {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(RIGHT_WIDTH, RIGHT_HEIGHT - 40.0)
                .gap(4.0),
        )
        .class("p-2 bg-surface")
        .child(body)
}

fn format_metadata(file: &MockFile) -> String {
    let mut out = String::with_capacity(256);
    out.push_str("Path:     ");
    out.push_str(file.path);
    out.push('\n');
    let _ = writeln!(out, "Size:     {} bytes", file.size);
    out.push_str("Kind:     ");
    out.push_str(file.kind);
    out.push('\n');
    out.push_str("Modified: ");
    out.push_str(file.modified);
    out.push('\n');
    out
}
