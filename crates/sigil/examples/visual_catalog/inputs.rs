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

fn invalid(state: State) -> Signal<bool> {
    signal(state == State::Invalid)
}

fn text_field_row(state: State) -> NodeBuilder {
    node(
        crate::widget_fields::text_field_demo()
            .disabled(off(state))
            .invalid_when(invalid(state)),
    )
}

fn field_row(_state: State) -> NodeBuilder {
    node(crate::widget_fields::field_demo())
}

fn validated_row(state: State) -> NodeBuilder {
    let error = (state == State::Invalid).then(|| String::from("Enter a valid email address"));
    node(crate::widget_fields::validated_field_demo(error))
}

fn text_edit_row(state: State) -> NodeBuilder {
    node(crate::widget_fields::text_edit_demo().disabled(off(state)))
}

fn number_row(state: State) -> NodeBuilder {
    node(
        crate::widget_fields::number_field_demo()
            .disabled(off(state))
            .invalid_when(invalid(state)),
    )
}

fn date_row(state: State) -> NodeBuilder {
    node(
        crate::widget_fields::date_field_demo()
            .disabled(off(state))
            .invalid_when(invalid(state)),
    )
}

fn color_row(state: State) -> NodeBuilder {
    node(
        crate::widget_fields::color_field_demo()
            .disabled(off(state))
            .invalid_when(invalid(state)),
    )
}

fn dropdown_row(state: State) -> NodeBuilder {
    node(crate::widget_fields::dropdown_demo().disabled(off(state)))
}

fn combobox_row(state: State) -> NodeBuilder {
    node(crate::widget_fields::combobox_demo(off(state)))
}

fn code_edit_row(state: State) -> NodeBuilder {
    node(crate::widget_documents::code_edit_demo().disabled(off(state)))
}

fn markdown_edit_row(state: State) -> NodeBuilder {
    crate::widget_documents::markdown_edit_demo(off(state))
}
