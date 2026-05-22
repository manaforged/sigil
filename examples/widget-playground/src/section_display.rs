//! Display widgets: `text`, `label`, `chip` (5 tones),
//! `divider_horizontal`, `divider_vertical`, `kv_row`, `status_bar`,
//! `progress`.

use sigil::prelude::*;
use sigil::widgets::patterns;
use sigil::widgets::{
    ChipTone, divider_horizontal, divider_vertical, kv_row, progress, status_bar,
};

use crate::{category, column, row};

pub(crate) fn display_section() -> NodeBuilder {
    let body = column(vec![
        text_row(),
        chips_row(),
        dividers_row(),
        kv_row_demo(),
        status_bar_demo(),
        progress_row(),
    ]);
    category("Display widgets", body)
}

fn text_row() -> NodeBuilder {
    row(vec![
        patterns::field("text", text("Plain text node")),
        patterns::field("label", label("Label widget")),
    ])
}

fn chips_row() -> NodeBuilder {
    // Natural-pack row of small chips — NOT the equal-cells `row()`
    // helper, which would stretch each chip to fill 1/5 of the parent.
    let chips = container()
        .layout(
            LayoutStyle::flex_row()
                .gap(8.0)
                .align_items(AlignItems::Center),
        )
        .child(chip("NEUTRAL", ChipTone::Neutral))
        .child(chip("INFO", ChipTone::Info))
        .child(chip("SUCCESS", ChipTone::Success))
        .child(chip("WARN", ChipTone::Warn))
        .child(chip("ERROR", ChipTone::Error));
    patterns::field("chip (all 5 tones)", chips)
}

fn dividers_row() -> NodeBuilder {
    let horizontal_demo = container()
        .layout(LayoutStyle::flex_column().gap(4.0).width(220.0))
        .child(text("above"))
        .child(divider_horizontal())
        .child(text("below"));
    let vertical_demo = container()
        .layout(
            LayoutStyle::flex_row()
                .gap(8.0)
                .height(40.0)
                .align_items(AlignItems::Center),
        )
        .child(text("left"))
        .child(divider_vertical())
        .child(text("right"));
    row(vec![
        patterns::field("divider_horizontal", horizontal_demo),
        patterns::field("divider_vertical", vertical_demo),
    ])
}

fn kv_row_demo() -> NodeBuilder {
    patterns::field(
        "kv_row",
        container()
            .layout(LayoutStyle::flex_column().gap(4.0).fill_width())
            .child(kv_row("Status", text("200 OK")))
            .child(kv_row("Length", text("1024 B")))
            .child(kv_row("Encoding", text("utf-8"))),
    )
}

fn status_bar_demo() -> NodeBuilder {
    // `status_bar` iterates a homogeneous `I::Item`; `label()` is a
    // `TextBuilder` and `chip()` is a `NodeBuilder`, so wrap both in
    // plain `container().child(...)` to unify the slot type.
    patterns::field(
        "status_bar",
        status_bar(vec![
            crate::wrap(label("Ready")),
            crate::wrap(chip("ONLINE", ChipTone::Success)),
            crate::wrap(label("23 items")),
        ]),
    )
}

fn progress_row() -> NodeBuilder {
    let value = signal(0.65_f32);
    let indeterminate = signal(0.0_f32);
    row(vec![
        patterns::field("progress (0.65)", progress(value, 1.0)),
        patterns::field(
            "progress (indeterminate)",
            progress(indeterminate, 1.0).indeterminate(),
        ),
    ])
}
