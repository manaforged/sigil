//! Container widgets: panel, scroll (nested), splitter, modal (with a
//! button that opens it).

use sigil::prelude::*;
use sigil::widgets::patterns;
use sigil::widgets::{SplitterOrientation, modal, panel, scroll, splitter};

use crate::{category, column, row};

pub(crate) fn container_section() -> NodeBuilder {
    let body = column(vec![
        panel_row(),
        nested_scroll_row(),
        splitter_row(),
        modal_row(),
    ]);
    category("Container widgets", body)
}

fn panel_row() -> NodeBuilder {
    let widget = panel()
        .title("Panel title")
        .padding(12.0)
        .elevation(1)
        .child(text("Panel body content").class("text-sm"));
    patterns::field("panel", widget)
}

fn nested_scroll_row() -> NodeBuilder {
    let mut inner = container().layout(LayoutStyle::flex_column().gap(4.0));
    for i in 0..40 {
        inner = inner.child(text(format!("scroll row {i}")).class("text-sm"));
    }
    let widget = scroll(inner, 800.0).width(280.0).height(160.0);
    patterns::field("scroll (nested, 40 rows)", widget)
}

fn splitter_row() -> NodeBuilder {
    let left = container()
        .layout(LayoutStyle::flex_column().padding(8.0).fill())
        .class("bg-accent")
        .child(text("left pane").class("text-on-accent"));
    let right = container()
        .layout(LayoutStyle::flex_column().padding(8.0).fill())
        .class("bg-surface-pressed")
        .child(text("right pane"));
    let widget = container()
        .layout(LayoutStyle::flex_row().fixed_size(560.0, 120.0))
        .child(
            splitter(SplitterOrientation::Horizontal)
                .child(left, 200.0)
                .min_size(0, 80.0)
                .max_size(0, 360.0)
                .child(right, 360.0),
        );
    patterns::field("splitter (horizontal)", widget)
}

fn modal_row() -> NodeBuilder {
    let open = signal(false);
    let trigger = button("Open modal")
        .primary()
        .activate(move || open.set(true));
    let dismiss = button("Close").activate(move || open.set(false));
    let dialog = modal()
        .show(open)
        .title("Demo dialog")
        .dismiss_on_backdrop(true)
        .child(text("This is a modal dialog body."))
        .child(dismiss);
    let host = container()
        .layout(
            LayoutStyle::flex_row()
                .gap(12.0)
                .align_items(AlignItems::Center),
        )
        .child(trigger)
        .child(text("(click to open the modal)").class("text-sm"))
        .child(dialog);
    patterns::field("modal (button-triggered)", row(vec![host]))
}
