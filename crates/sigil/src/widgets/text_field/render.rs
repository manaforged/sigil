use sigil_events::prelude::{NodeEventsExt, PointerEvent};
use sigil_kernel::{NodeBuilder, Role, TextBuilder, TextContent, container, text};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle, LengthPercentage};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::{NodeStyleExt, StateStyle, StyleAtom, StyleVariant};

use crate::interaction::InteractionState;

use super::super::shared::binary_style_state;

static FIELD_BASE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("border")];
static FIELD_REST: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("border")];
static FIELD_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-surface")];
static FIELD_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-surface")];
static FIELD_DOWN: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("on-surface"),
    StyleAtom::Opacity(8_800),
];
static FIELD_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("border"),
    StyleAtom::Opacity(5_000),
];

pub(super) fn build_base_node(
    value: Signal<String>,
    disabled: Option<Signal<bool>>,
    placeholder: &TextContent,
    interaction: InteractionState,
) -> NodeBuilder {
    let placeholder_str = text_content_untracked(placeholder);
    let displayed = displayed_text(value, interaction.focus_visible(), placeholder.clone());
    container()
        .role(Role::TextInput)
        .aria_label(accessible_label(placeholder_str.as_str()))
        .aria_text_value_signal(value)
        .aria_placeholder(placeholder_str)
        .layout(field_layout())
        .class("rounded-md")
        .style_reactive(move || field_variant().state(field_state(interaction, disabled)))
        .on_pointer_up(move |event| pointer_up(event, interaction))
        .child(field_inner(displayed))
}

fn field_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .width_px(180.0)
        .align_items(AlignItems::Center)
        .with_taffy(|style| {
            style.padding.left = LengthPercentage::length(1.0);
            style.padding.right = LengthPercentage::length(1.0);
            style.padding.top = LengthPercentage::length(1.0);
            style.padding.bottom = LengthPercentage::length(1.0);
        })
}

fn field_inner(displayed: Signal<String>) -> NodeBuilder {
    container()
        .layout(field_inner_layout())
        .class("rounded-sm")
        .class("bg-surface")
        .child(field_text(displayed))
}

fn field_inner_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .width_px(178.0)
        .align_items(AlignItems::Center)
        .with_taffy(|style| {
            style.padding.left = LengthPercentage::length(12.0);
            style.padding.right = LengthPercentage::length(12.0);
            style.padding.top = LengthPercentage::length(8.0);
            style.padding.bottom = LengthPercentage::length(8.0);
        })
}

fn field_text(displayed: Signal<String>) -> TextBuilder {
    text(displayed)
        .layout(LayoutStyle::block().width_px(156.0))
        .class("text-on-surface")
}

fn pointer_up(event: &mut PointerEvent, interaction: InteractionState) {
    let _ = interaction.release_pointer(event);
}

fn displayed_text(
    value: Signal<String>,
    focused: Signal<bool>,
    placeholder: TextContent,
) -> Signal<String> {
    derived_eq(move || {
        let current = value.get();
        if current.is_empty() && !focused.get() {
            text_content(&placeholder)
        } else {
            current
        }
    })
}

fn text_content(content: &TextContent) -> String {
    match content {
        TextContent::Static(value) => value.clone(),
        TextContent::Reactive(value) => value.get(),
    }
}

fn text_content_untracked(content: &TextContent) -> String {
    match content {
        TextContent::Static(value) => value.clone(),
        TextContent::Reactive(value) => value.peek(),
    }
}

fn accessible_label(placeholder: &str) -> String {
    (!placeholder.is_empty())
        .then_some(placeholder)
        .filter(|text| !text.is_empty())
        .map_or_else(|| String::from("Text field"), ToOwned::to_owned)
}

fn field_state(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> sigil_style::StyleState {
    binary_style_state(interaction, disabled)
}

const fn field_variant() -> StyleVariant {
    StyleVariant::new(
        FIELD_BASE,
        StateStyle::new(
            FIELD_REST,
            FIELD_FOCUS,
            FIELD_HOVER,
            FIELD_DOWN,
            FIELD_DISABLED,
        ),
    )
}
