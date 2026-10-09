#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{
    ColorFieldBuilder, DateFieldBuilder, DropdownBuilder, FieldBuilder, NumberFieldBuilder,
    TextEditBuilder, TextFieldBuilder,
};

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - fields").size(520, 720), build)
}

pub(crate) fn text_field_demo() -> TextFieldBuilder {
    text_field(signal(String::from("Ada Lovelace"))).placeholder("Full name")
}

pub(crate) fn text_edit_demo() -> TextEditBuilder {
    text_edit(signal(String::from("Multi-line\neditable text."))).height(96.0)
}

pub(crate) fn number_field_demo() -> NumberFieldBuilder<i32> {
    number_field(signal(4_i32), 1..=99)
}

pub(crate) fn date_field_demo() -> DateFieldBuilder {
    date_field(signal(Date {
        year: 2026,
        month: 4,
        day: 1,
    }))
}

pub(crate) fn color_field_demo() -> ColorFieldBuilder {
    color_field(signal(Rgba8 {
        r: 90,
        g: 140,
        b: 255,
        a: 255,
    }))
}

pub(crate) fn combobox_demo(disabled: Signal<bool>) -> NodeBuilder {
    let language = Combobox::new(vec![
        String::from("Rust"),
        String::from("Go"),
        String::from("Zig"),
        String::from("C++"),
    ])
    .placeholder("Language")
    .disabled(disabled);
    combobox(language)
}

pub(crate) fn dropdown_demo() -> DropdownBuilder {
    dropdown(
        signal(1_usize),
        vec!["Small".into(), "Medium".into(), "Large".into()],
    )
}

pub(crate) fn field_demo() -> FieldBuilder<TextFieldBuilder> {
    field(
        "Name",
        text_field(signal(String::new())).placeholder("Full name"),
    )
}

pub(crate) fn validated_field_demo(error: Option<String>) -> FieldBuilder<TextFieldBuilder> {
    field(
        "Email",
        text_field(signal(String::from("ada@"))).placeholder("name@example.com"),
    )
    .error(signal(error))
}

fn build() -> impl IntoNode {
    scroll(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
            .child(text_field_demo())
            .child(text_edit_demo())
            .child(number_field_demo())
            .child(date_field_demo())
            .child(color_field_demo())
            .child(combobox_demo(signal(false)))
            .child(dropdown_demo())
            .child(field_demo())
            .child(validated_field_demo(None)),
    )
    .fill()
}
