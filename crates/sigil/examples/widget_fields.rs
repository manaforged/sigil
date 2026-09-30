#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{
    ColorFieldBuilder, DateFieldBuilder, DropdownBuilder, NumberFieldBuilder, TextEditBuilder,
    TextFieldBuilder,
};

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - fields").size(520, 720), build)
}

fn text_field_demo() -> TextFieldBuilder {
    let name = signal(String::new());
    text_field(name).placeholder("Full name")
}

fn text_edit_demo() -> TextEditBuilder {
    let notes = signal(String::from("First line\nSecond line"));
    text_edit(notes).placeholder("Notes")
}

fn number_field_demo() -> NumberFieldBuilder<i32> {
    let quantity = signal(1_i32);
    number_field(quantity, 1..=99)
}

fn date_field_demo() -> DateFieldBuilder {
    let due = signal(Date {
        year: 2026,
        month: 9,
        day: 24,
    });
    date_field(due)
}

fn color_field_demo() -> ColorFieldBuilder {
    let accent = signal(Rgba8 {
        r: 90,
        g: 140,
        b: 255,
        a: 255,
    });
    color_field(accent)
}

fn combobox_demo() -> NodeBuilder {
    let language = Combobox::new(vec![
        String::from("Rust"),
        String::from("Go"),
        String::from("Zig"),
    ])
    .placeholder("Language");
    combobox(language)
}

fn dropdown_demo() -> DropdownBuilder {
    let size = signal(0_usize);
    dropdown(size, vec!["Small".into(), "Large".into()]).label("Size")
}

fn field_demo() -> impl IntoNode {
    let city = signal(String::new());
    field("City", text_field(city).placeholder("Berlin"))
}

fn validated_field_demo() -> impl IntoNode {
    let email = signal(String::from("ada@"));
    field("Email", text_field(email).placeholder("name@example.com")).error(move || {
        (!email.get().contains('.')).then(|| String::from("Enter a valid email address"))
    })
}

fn build() -> impl IntoNode {
    scroll(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
            .child(field("Name", text_field_demo()))
            .child(field("Notes", text_edit_demo()))
            .child(field("Quantity", number_field_demo()))
            .child(field("Due", date_field_demo()))
            .child(field("Accent", color_field_demo()))
            .child(field("Language", combobox_demo()))
            .child(field("Size", dropdown_demo()))
            .child(field_demo())
            .child(validated_field_demo()),
    )
    .fill()
}
