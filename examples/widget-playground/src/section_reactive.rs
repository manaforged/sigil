//! Reactive demos: `virtual_list` with `items_bind` (push/pop),
//! `text_edit` that mutates a keystroke counter signal, `chip` whose
//! tone flips on a toggle.

use sigil::prelude::*;
use sigil::widgets::patterns;
use sigil::widgets::{ChipTone, chip, virtual_list};

use crate::{category, column, row};

pub(crate) fn reactive_section() -> NodeBuilder {
    let body = column(vec![
        virtual_list_row(),
        keystroke_counter_row(),
        reactive_chip_row(),
    ]);
    category("Reactive demos", body)
}

fn virtual_list_row() -> NodeBuilder {
    let items = signal((0..50_u32).collect::<Vec<_>>());
    let push = button("Push").activate(move || {
        items.update(|v| {
            let next = v.last().copied().map_or(0, |n| n.saturating_add(1));
            v.push(next);
        });
    });
    let pop = button("Pop").activate(move || {
        items.update(|v| {
            v.pop();
        });
    });
    let list = virtual_list(items.peek(), |idx, n: &u32| {
        crate::wrap(text(format!("row {idx}: {n}")).class("text-sm"))
    })
    .row_height(22.0)
    .viewport_height(180.0)
    .viewport_width(280.0)
    .items_bind(items);

    let controls = container()
        .layout(LayoutStyle::flex_row().gap(8.0))
        .child(push)
        .child(pop);
    let stack = container()
        .layout(LayoutStyle::flex_column().gap(8.0))
        .child(controls)
        .child(list);
    patterns::field("virtual_list (push/pop, items_bind)", stack)
}

fn keystroke_counter_row() -> NodeBuilder {
    let buffer = signal(String::new());
    let count = signal(0_u32);
    let count_label = signal(format!("Keystrokes: {}", count.peek()));
    effect(move || count_label.set(format!("Keystrokes: {}", count.get())));

    let field = text_edit(buffer)
        .placeholder("type to increment...")
        .change(move |_| {
            count.update(|n| *n = n.saturating_add(1));
        });
    let stack = container()
        .layout(LayoutStyle::flex_column().gap(6.0))
        .child(field)
        .child(label(count_label));
    patterns::field("text_edit + keystroke counter", stack)
}

fn reactive_chip_row() -> NodeBuilder {
    let success = signal(true);
    let tone_label = signal(String::from("Success"));
    effect(move || {
        tone_label.set(if success.get() {
            String::from("Success")
        } else {
            String::from("Error")
        });
    });

    let toggle_widget = toggle(success).label("flip tone (Success / Error)");
    // `chip` returns `NodeBuilder` (no builder type), so reactive tone
    // flips by mounting / unmounting the two chip instances via a
    // `children_reactive` host container.
    let chip_host = container()
        .layout(LayoutStyle::flex_row())
        .children_reactive(move || {
            let tone = if success.get() {
                ChipTone::Success
            } else {
                ChipTone::Error
            };
            let label_text = if success.get() { "SUCCESS" } else { "ERROR" };
            vec![chip(label_text, tone)]
        });
    let stack = container()
        .layout(LayoutStyle::flex_column().gap(6.0))
        .child(toggle_widget)
        .child(chip_host)
        .child(label(tone_label));
    patterns::field("chip (reactive tone flip)", row(vec![stack]))
}
