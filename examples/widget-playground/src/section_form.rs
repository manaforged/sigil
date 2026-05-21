//! Form widgets: `button`, `text_field`, `text_edit`, `checkbox`,
//! `toggle`, `slider`, `dropdown`, `number_field`. One instance each
//! at default state, with disabled variants rendered side-by-side for
//! the widgets where that's interesting.

use sigil::prelude::*;
use sigil::widgets::patterns;

use crate::{category, column, row};

pub(crate) fn form_section() -> NodeBuilder {
    let body = column(vec![
        buttons_row(),
        text_inputs_row(),
        binary_inputs_row(),
        slider_row(),
        dropdown_row(),
        number_field_row(),
    ]);
    category("Form widgets", body)
}

fn buttons_row() -> NodeBuilder {
    let disabled = signal(true);
    row(vec![
        patterns::field("Button (neutral)", button("Save")),
        patterns::field("Button (primary)", button("Save").primary()),
        patterns::field("Button (disabled)", button("Save").disabled(disabled)),
    ])
}

fn text_inputs_row() -> NodeBuilder {
    let value = signal(String::from("hello"));
    let disabled_value = signal(String::from("read-only"));
    let disabled = signal(true);
    let multiline = signal(String::from("multi\nline\ncontent"));
    row(vec![
        patterns::field(
            "text_field",
            text_field(value).placeholder("type something"),
        ),
        patterns::field(
            "text_field (disabled)",
            text_field(disabled_value).disabled(disabled),
        ),
        patterns::field(
            "text_edit (multi-line)",
            text_edit(multiline).placeholder("notes..."),
        ),
    ])
}

fn binary_inputs_row() -> NodeBuilder {
    let on = signal(true);
    let off = signal(false);
    let disabled = signal(true);
    let toggle_on = signal(true);
    row(vec![
        patterns::field("checkbox (on)", checkbox(on).label("enabled")),
        patterns::field(
            "checkbox (disabled)",
            checkbox(off).label("disabled").disabled(disabled),
        ),
        patterns::field("toggle", toggle(toggle_on).label("notifications")),
    ])
}

fn slider_row() -> NodeBuilder {
    let value = signal(0.4_f32);
    let disabled_value = signal(0.7_f32);
    let disabled = signal(true);
    row(vec![
        patterns::field("slider", slider(value, 0.0..=1.0).step(0.05)),
        patterns::field(
            "slider (disabled)",
            slider(disabled_value, 0.0..=1.0).disabled(disabled),
        ),
    ])
}

fn dropdown_row() -> NodeBuilder {
    let selected = signal(1_usize);
    let options = vec![
        String::from("Apple"),
        String::from("Banana"),
        String::from("Cherry"),
    ];
    patterns::field("dropdown", dropdown(selected, options))
}

fn number_field_row() -> NodeBuilder {
    let n = signal(42_i32);
    patterns::field("number_field (0..=100)", number_field(n, 0..=100))
}
