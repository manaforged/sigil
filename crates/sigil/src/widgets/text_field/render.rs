use sigil_animation_runtime::presets;
use sigil_events::prelude::{NodeEventsExt, PointerEvent};
use sigil_kernel::{NodeBuilder, Role, TextBuilder, TextContent, container, text};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::{NodeStyleExt, StateStyle, StyleAtom, StyleVariant, TransitionId};

use crate::interaction::InteractionState;

use super::super::shared::binary_style_state;
use super::super::transition_seam::install_transition_animators;

const TEXT_FIELD_TRANSITION: TransitionId = presets::SMOOTH;
const TEXT_FIELD_COMPONENT_ID: u64 = 0x7E47_F000;
// Animator-only base slice scanned by `install_transition_animators`.
static TEXT_FIELD_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(TEXT_FIELD_TRANSITION)];

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
    let state_style = field_state_style();
    let node = container()
        .role(Role::TextInput)
        .label(accessible_label(placeholder_str.as_str()))
        .text_value(value)
        .placeholder(placeholder_str)
        .layout(field_layout())
        .class("rounded-md")
        .class(TEXT_FIELD_ANIM_BASE)
        .style_reactive(move || field_variant().state(field_state(interaction, disabled)))
        .pointer_up(move |event| pointer_up(event, interaction))
        .child(field_inner(displayed));
    install_transition_animators(
        node,
        TEXT_FIELD_ANIM_BASE,
        state_style,
        TEXT_FIELD_COMPONENT_ID,
        interaction,
    )
}

const fn field_state_style() -> StateStyle {
    StateStyle::new(
        FIELD_REST,
        FIELD_FOCUS,
        FIELD_HOVER,
        FIELD_DOWN,
        FIELD_DISABLED,
    )
}

fn field_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .width(180.0)
        .align_items(AlignItems::Center)
        .padding(1.0)
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
        .width(178.0)
        .align_items(AlignItems::Center)
        .padding_x(12.0)
        .padding_y(8.0)
}

fn field_text(displayed: Signal<String>) -> TextBuilder {
    text(displayed)
        .layout(LayoutStyle::block().width(156.0))
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
