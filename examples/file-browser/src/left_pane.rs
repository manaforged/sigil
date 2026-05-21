//! Left pane: a `virtual_list` of mock file paths with single-select.

use sigil::prelude::*;
use sigil::widgets::{SelectionMode, virtual_list};

use crate::files::MockFile;

/// Build the left pane.
///
/// `selected_index` is the host-owned selection bound — when a row click
/// updates the `virtual_list`'s `SelectionState`, `.select` mirrors the
/// chosen index into this signal so the right pane re-renders.
#[must_use]
pub(super) fn build(files: &[MockFile], selected_index: Signal<usize>) -> impl IntoNode + use<> {
    let labels: Vec<&'static str> = files.iter().map(|file| file.path).collect();

    let list = virtual_list(labels, move |index, path| {
        container()
            .role(Role::ListItem)
            .label(format!("Row {index}: {path}"))
            .layout(LayoutStyle::flex_row().fixed_size(360.0, 28.0).gap(8.0))
            .class("px-2")
            .child(
                text(*path)
                    .layout(LayoutStyle::block().width(344.0))
                    .class("text-on-surface text-sm"),
            )
    })
    .row_height(28.0)
    .viewport_height(540.0)
    .viewport_width(360.0)
    .selection(SelectionMode::Single)
    .select(move |index| selected_index.set(index));

    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fixed_size(360.0, 600.0).gap(4.0))
        .class("bg-surface")
        .child(
            text("Files")
                .layout(LayoutStyle::block().width(360.0))
                .class("text-on-surface text-base font-bold px-2 py-1"),
        )
        .child(list)
}
