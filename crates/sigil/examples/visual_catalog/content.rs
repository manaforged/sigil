use sigil::prelude::*;

use super::spec::{Build, State, node};

pub(crate) const BUILDERS: &[(&str, Build)] = &[
    ("text", text_row),
    ("label", label_row),
    ("section_label", section_row),
    ("icon", icon_row),
    ("chip", chip_row),
    ("dot", dot_row),
    ("avatar", avatar_row),
    ("path", path_row),
    ("image_rgba", image_row),
    ("divider_horizontal", divider_h_row),
    ("divider_vertical", divider_v_row),
    ("notice_band", notice_row),
    ("empty_state", empty_row),
    ("hint_bar", hint_bar_row),
    ("markdown", markdown_row),
    ("markdown_image", markdown_image_row),
    ("mermaid", mermaid_row),
    ("html", html_row),
];

fn text_row(_state: State) -> NodeBuilder {
    node(crate::widget_content::text_demo())
}

fn label_row(_state: State) -> NodeBuilder {
    node(crate::widget_content::label_demo())
}

fn section_row(_state: State) -> NodeBuilder {
    node(crate::widget_content::section_label_demo())
}

fn icon_row(_state: State) -> NodeBuilder {
    node(crate::widget_content::icon_demo())
}

fn chip_row(_state: State) -> NodeBuilder {
    node(crate::widget_indicators::chip_demo())
}

fn dot_row(_state: State) -> NodeBuilder {
    crate::widget_indicators::dot_demo()
}

fn avatar_row(_state: State) -> NodeBuilder {
    node(crate::widget_content::avatar_demo())
}

fn path_row(_state: State) -> NodeBuilder {
    node(crate::widget_content::path_demo())
}

fn image_row(_state: State) -> NodeBuilder {
    crate::widget_content::image_rgba_demo()
}

fn divider_h_row(_state: State) -> NodeBuilder {
    node(crate::widget_content::divider_horizontal_demo())
}

fn divider_v_row(_state: State) -> NodeBuilder {
    crate::widget_content::divider_vertical_demo()
}

fn notice_row(_state: State) -> NodeBuilder {
    node(crate::widget_content::notice_band_demo())
}

fn empty_row(_state: State) -> NodeBuilder {
    node(crate::widget_content::empty_state_demo())
}

fn hint_bar_row(_state: State) -> NodeBuilder {
    node(crate::widget_content::hint_bar_demo())
}

fn markdown_row(_state: State) -> NodeBuilder {
    node(crate::widget_documents::markdown_demo())
}

fn markdown_image_row(_state: State) -> NodeBuilder {
    node(crate::widget_documents::markdown_image_demo())
}

fn mermaid_row(_state: State) -> NodeBuilder {
    node(crate::widget_documents::mermaid_demo())
}

fn html_row(_state: State) -> NodeBuilder {
    node(crate::widget_documents::html_demo())
}
