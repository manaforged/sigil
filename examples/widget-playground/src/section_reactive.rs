use sigil::prelude::*;
use sigil::widgets::chip;
use sigil::widgets::patterns;

use crate::{category, column, row};

pub(crate) fn reactive_section() -> NodeBuilder {
    let body = column(vec![
        virtual_list_row(),
        keystroke_counter_row(),
        reactive_chip_row(),
    ]);
    category("Reactive demos", body)
}

#[derive(Clone, PartialEq)]
struct DemoRow {
    id: u32,
    label: String,
}

impl Identifiable for DemoRow {
    type Id = u32;
    fn id(&self) -> Self::Id {
        self.id
    }
}

fn virtual_list_row() -> NodeBuilder {
    let items = signal(
        (0..50_u32)
            .map(|id| DemoRow {
                id,
                label: format!("row {id}"),
            })
            .collect::<Vec<_>>(),
    );
    let push = button("Push").activate(move || {
        items.update(|v| {
            let id = v.last().map_or(0, |row| row.id.saturating_add(1));
            v.push(DemoRow {
                id,
                label: format!("row {id}"),
            });
        });
    });
    let pop = button("Pop").activate(move || {
        items.update(|v| {
            v.pop();
        });
    });
    let list = virtual_list(items, |_, row: &DemoRow| {
        crate::wrap(text(row.label.clone()).class(classes!("text-sm")))
    })
    .identified()
    .row_height(22.0)
    .viewport_height(180.0)
    .viewport_width(280.0);

    let controls = container()
        .layout(LayoutStyle::flex_row().gap(8.0))
        .child(push)
        .child(pop);
    let stack = container()
        .layout(LayoutStyle::flex_column().gap(8.0))
        .child(controls)
        .child(list);
    patterns::field("virtual_list (live, identified)", stack)
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

    let tone = signal(Tone::Success);
    effect(move || {
        tone.set(if success.get() {
            Tone::Success
        } else {
            Tone::Error
        });
    });

    let toggle_widget = toggle(success).label("flip tone (Success / Error)");
    let chip_widget = chip("STATUS", Tone::Success).tone_signal(tone);
    let stack = container()
        .layout(LayoutStyle::flex_column().gap(6.0))
        .child(toggle_widget)
        .child(chip_widget)
        .child(label(tone_label));
    patterns::field("chip (reactive tone flip)", row(vec![stack]))
}
