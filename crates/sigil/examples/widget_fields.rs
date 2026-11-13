#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{Combobox, Date, TextFieldBuilder};

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - fields").size(520, 720), build)
}

pub(crate) fn text_field_demo() -> TextFieldBuilder {
    text_field(signal(String::from("Ada Lovelace"))).placeholder("Full name")
}

pub(crate) fn text_edit_demo() -> TextFieldBuilder {
    text_field(signal(String::from("Multi-line\neditable text.")))
        .multiline()
        .height(96.0)
}

pub(crate) fn number_field_demo(disabled: Signal<bool>, invalid: Signal<bool>) -> impl IntoNode {
    number_field(signal(4_i32), 1..=99)
        .disabled(disabled)
        .invalid(invalid)
}

pub(crate) fn date_field_demo(disabled: Signal<bool>, invalid: Signal<bool>) -> impl IntoNode {
    date_field(signal(Date::new(2026, 4, 1).expect("a calendar date")))
        .disabled(disabled)
        .invalid(invalid)
}

pub(crate) fn color_field_demo(disabled: Signal<bool>, invalid: Signal<bool>) -> impl IntoNode {
    color_field(signal(Color {
        r: 90,
        g: 140,
        b: 255,
        a: 255,
    }))
    .disabled(disabled)
    .invalid(invalid)
}

pub(crate) fn combobox_demo(disabled: Signal<bool>) -> NodeBuilder {
    let language = Combobox::new(["Rust", "Go", "Zig", "C++"])
        .placeholder("Language")
        .disabled(disabled);
    combobox(language)
}

pub(crate) fn dropdown_demo(disabled: Signal<bool>) -> impl IntoNode {
    dropdown(signal(1_usize), ["Small", "Medium", "Large"]).disabled(disabled)
}

pub(crate) fn field_demo() -> impl IntoNode {
    field(
        "Name",
        text_field(signal(String::new())).placeholder("Full name"),
    )
}

pub(crate) fn validated_field_demo(error: Option<String>) -> impl IntoNode {
    field(
        "Email",
        text_field(signal(String::from("ada@"))).placeholder("name@example.com"),
    )
    .error(signal(error))
}

fn build() -> impl IntoNode {
    scroll(
        column()
            .role(Role::Main)
            .gap(rows(1.0))
            .padding(cols(2.0))
            .child(text_field_demo())
            .child(text_edit_demo())
            .child(number_field_demo(signal(false), signal(false)))
            .child(date_field_demo(signal(false), signal(false)))
            .child(color_field_demo(signal(false), signal(false)))
            .child(combobox_demo(signal(false)))
            .child(dropdown_demo(signal(false)))
            .child(field_demo())
            .child(validated_field_demo(None)),
    )
    .fill()
}
