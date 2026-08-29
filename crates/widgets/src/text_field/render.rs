use std::cell::Cell;
use std::rc::Rc;

use sigil_animation_runtime::presets;
use sigil_events::prelude::{NodeEventsExt, PointerEvent};
use sigil_kernel::{NodeBuilder, Role, TextContent, container};
use sigil_layout::TextStyleCapture;
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::{
    ColorAtom, NodeStyleExt, Px, ReactiveStateSlot, Rgba8, StateSlot, StateStyle, StyleAtom,
    StyleOverlay, StyleVariant, TransitionId,
};

use crate::interaction::InteractionState;

use super::handlers::TextFieldRuntime;

use super::super::shared::binary_style_state;
use super::super::transition_seam::install_transition_animators;

const TEXT_FIELD_TRANSITION: TransitionId = presets::SMOOTH;
const TEXT_FIELD_COMPONENT_ID: u64 = 0x7E47_F000;
static TEXT_FIELD_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(TEXT_FIELD_TRANSITION)];

static FIELD_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("field"),
    StyleAtom::BorderWidth(Px(1)),
];
static FIELD_REST: &[StyleAtom] = &[StyleAtom::BorderToneStep(7)];
static FIELD_FOCUS: &[StyleAtom] = &[StyleAtom::BorderThemeRef("accent")];
static FIELD_HOVER: &[StyleAtom] = &[StyleAtom::BorderToneStep(8)];
static FIELD_DOWN: &[StyleAtom] = &[StyleAtom::BorderThemeRef("accent")];
static FIELD_DISABLED: &[StyleAtom] = &[StyleAtom::BorderToneStep(6), StyleAtom::Opacity(5_000)];
pub(super) static FIELD_INVALID: &[StyleAtom] =
    &[StyleAtom::BorderColor(ColorAtom::Rgba8(Rgba8 {
        r: 211,
        g: 47,
        b: 47,
        a: 255,
    }))];

#[derive(Clone)]
pub(super) struct FieldInputs {
    pub(super) value: Signal<String>,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) invalid: Option<Signal<bool>>,
    pub(super) overlay: StyleOverlay,
    pub(super) width: Option<f32>,
    pub(super) height: Option<f32>,
}

pub(super) fn build_base_node(
    inputs: FieldInputs,
    placeholder: &TextContent,
    interaction: InteractionState,
    runtime: TextFieldRuntime,
) -> NodeBuilder {
    let FieldInputs {
        value,
        disabled,
        invalid,
        overlay,
        width,
        height,
    } = inputs;
    let placeholder_str = text_content_untracked(placeholder);
    let displayed = displayed_text(
        value,
        placeholder.clone(),
        runtime.ime.preedit,
        runtime.caret,
    );
    let state_style = field_state_style();
    let slot_cell: ReactiveStateSlot = Rc::new(Cell::new(StateSlot::Rest));
    let channel = field_style_channel(interaction, disabled, invalid, Rc::clone(&slot_cell));
    let text_style = runtime.text_style;
    let node = container()
        .mutate(move |node| {
            let _ = node.attach(TextStyleCapture::new(text_style));
        })
        .role(Role::TextInput)
        .label(accessible_label(placeholder_str.as_str()))
        .text_value(value)
        .placeholder(placeholder_str)
        .layout(field_layout())
        .class("rounded-md")
        .class(FIELD_BASE)
        .class(TEXT_FIELD_ANIM_BASE)
        .style_reactive(channel)
        .state_overlay(overlay, slot_cell)
        .pointer_up(move |event| pointer_up(event, interaction))
        .child(field_inner(runtime, interaction, displayed, width, height));
    install_transition_animators(
        node,
        TEXT_FIELD_ANIM_BASE,
        state_style,
        TEXT_FIELD_COMPONENT_ID,
        interaction,
    )
}

fn field_style_channel(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    invalid: Option<Signal<bool>>,
    slot_cell: ReactiveStateSlot,
) -> impl Fn() -> &'static [StyleAtom] {
    move || {
        let state = field_state(interaction, disabled);
        slot_cell.set(state.slot());
        if invalid.is_some_and(Signal::get) {
            FIELD_INVALID
        } else {
            field_variant().state(state)
        }
    }
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

pub(super) fn field_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_width()
        .min_size(0.0, sigil_layout::prelude::Dimension::Auto)
        .align_items(AlignItems::Center)
}

fn field_inner(
    runtime: TextFieldRuntime,
    interaction: InteractionState,
    displayed: Signal<String>,
    width: Option<f32>,
    height: Option<f32>,
) -> NodeBuilder {
    container()
        .layout(field_inner_layout(width, height))
        .child(super::selection::text_region(
            runtime,
            interaction,
            displayed,
        ))
}

fn field_inner_layout(width: Option<f32>, height: Option<f32>) -> LayoutStyle {
    let base = width.map_or_else(
        || LayoutStyle::flex_row().fill_width().grow(1.0),
        |w| LayoutStyle::flex_row().width((w - 2.0).max(0.0)),
    );
    let base = match height {
        Some(h) => base.height((h - 2.0).max(0.0)),
        None => base,
    };
    base.align_items(AlignItems::Center)
        .padding_x(12.0)
        .padding_y(6.0)
}

fn pointer_up(event: &mut PointerEvent, interaction: InteractionState) {
    let _ = interaction.release_pointer(event);
}

fn displayed_text(
    value: Signal<String>,
    placeholder: TextContent,
    preedit: Signal<String>,
    caret: Signal<usize>,
) -> Signal<String> {
    derived_eq(move || {
        let current = value.get();
        let composing = preedit.get();
        if !composing.is_empty() {
            splice_preedit(&current, caret.get(), &composing)
        } else if current.is_empty() {
            text_content(&placeholder)
        } else {
            current
        }
    })
}

fn splice_preedit(value: &str, caret: usize, preedit: &str) -> String {
    let caret = crate::text_edit::floor_char_boundary(value, caret.min(value.len()));
    let mut out = String::with_capacity(value.len() + preedit.len());
    out.push_str(&value[..caret]);
    out.push_str(preedit);
    out.push_str(&value[caret..]);
    out
}

fn text_content(content: &TextContent) -> String {
    content.resolve()
}

fn text_content_untracked(content: &TextContent) -> String {
    content.resolve()
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
