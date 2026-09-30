use sigil::prelude::*;

use super::spec::{Build, State, node, off};

pub(crate) const BUILDERS: &[(&str, Build)] = &[
    ("text_field", text_field_row),
    ("field", field_row),
    ("validated_field", validated_row),
    ("text_edit", text_edit_row),
    ("number_field", number_row),
    ("date_field", date_row),
    ("color_field", color_row),
    ("dropdown", dropdown_row),
    ("combobox", combobox_row),
    ("code_edit", code_edit_row),
    ("markdown_edit", markdown_edit_row),
];

fn text_field_row(state: State) -> NodeBuilder {
    node(
        text_field(signal(String::from("Ada Lovelace")))
            .placeholder("Full name")
            .disabled(off(state))
            .invalid_when(signal(state == State::Invalid)),
    )
}

fn field_row(_state: State) -> NodeBuilder {
    node(field(
        "Name",
        text_field(signal(String::new())).placeholder("Full name"),
    ))
}

fn validated_row(state: State) -> NodeBuilder {
    let error = (state == State::Invalid).then(|| String::from("Enter a valid email address"));
    node(
        field(
            "Email",
            text_field(signal(String::from("ada@"))).placeholder("name@example.com"),
        )
        .error(signal(error)),
    )
}

fn text_edit_row(state: State) -> NodeBuilder {
    node(
        text_edit(signal(String::from("Multi-line\neditable text.")))
            .height(96.0)
            .disabled(off(state)),
    )
}

fn invalid(state: State) -> Signal<bool> {
    signal(state == State::Invalid)
}

fn number_row(state: State) -> NodeBuilder {
    node(
        number_field(signal(4_i32), 1..=99)
            .disabled(off(state))
            .invalid_when(invalid(state)),
    )
}

fn date_row(state: State) -> NodeBuilder {
    let day = Date {
        year: 2026,
        month: 4,
        day: 1,
    };
    node(
        date_field(signal(day))
            .disabled(off(state))
            .invalid_when(invalid(state)),
    )
}

fn color_row(state: State) -> NodeBuilder {
    let colour = Rgba8 {
        r: 90,
        g: 140,
        b: 255,
        a: 255,
    };
    node(
        color_field(signal(colour))
            .disabled(off(state))
            .invalid_when(invalid(state)),
    )
}

fn dropdown_row(state: State) -> NodeBuilder {
    node(
        dropdown(
            signal(1_usize),
            vec!["Small".into(), "Medium".into(), "Large".into()],
        )
        .disabled(off(state)),
    )
}

fn combobox_row(state: State) -> NodeBuilder {
    let combo = Combobox::new(vec![
        String::from("Rust"),
        String::from("Go"),
        String::from("Zig"),
        String::from("C++"),
    ])
    .placeholder("Language")
    .disabled(off(state));
    node(combobox(combo))
}

fn code_edit_row(state: State) -> NodeBuilder {
    let lines = vec![
        String::from("fn main() {"),
        String::from("    println!(\"hi\");"),
        String::from("}"),
    ];
    node(code_edit(signal(lines), signal((1, 4))).disabled(off(state)))
}

fn markdown_edit_row(state: State) -> NodeBuilder {
    let source = signal(String::from(
        "# Sigil widgets\n\nEdit me, *live* preview.\n\n- one\n- two",
    ));
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(228.0))
        .child(markdown_edit(source).disabled(off(state)))
}
