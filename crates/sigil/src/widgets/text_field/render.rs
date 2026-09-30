use std::cell::Cell;
use std::rc::Rc;

use crate::animation_runtime::presets;
use crate::events::{NodeEventsExt, PointerEvent};
use crate::kernel::{NodeBuilder, Role, TextContent, container};
use crate::layout::TextStyleCapture;
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived};
use crate::style::{
    ColorAtom, ReactiveStateSlot, Rgba8, StateSlot, StyleAtom, StyleOverlay, TransitionId, space,
};

use crate::widgets::interaction::InteractionState;

use super::handlers::TextFieldRuntime;

use super::super::shared::{FIELD_VARIANT, binary_style_state};

const TEXT_FIELD_TRANSITION: TransitionId = presets::SMOOTH;
static TEXT_FIELD_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(TEXT_FIELD_TRANSITION)];

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
    let displayed = displayed_text(value, placeholder.clone(), &runtime);
    let slot_cell: ReactiveStateSlot = Rc::new(Cell::new(StateSlot::Rest));
    let channel = field_style_channel(interaction, disabled, invalid, Rc::clone(&slot_cell));
    let text_style = runtime.input.text_style;
    let mut node = container()
        .mutate(move |node| {
            let _ = node.attach(TextStyleCapture::new(text_style));
        })
        .role(if runtime.password {
            Role::PasswordInput
        } else {
            Role::TextInput
        })
        .label(accessible_label(placeholder_str.as_str()));
    if !runtime.password {
        node = node
            .text_value(value)
            .text_cursor(runtime.input.caret, runtime.input.selection);
    }
    node.placeholder(placeholder_str)
        .layout(field_layout())
        .class(FIELD_VARIANT.base())
        .class(TEXT_FIELD_ANIM_BASE)
        .style_reactive(channel)
        .state_overlay(overlay, slot_cell)
        .seal_style()
        .pointer_up(move |event| pointer_up(event, interaction))
        .child(field_inner(runtime, interaction, displayed, width, height))
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
            FIELD_VARIANT.state(state)
        }
    }
}

pub(super) fn field_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_width()
        .min_size(0.0, crate::layout::Dimension::Auto)
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
    let inset = crate::layout::snap::line(1.0);
    let padding_y = crate::layout::snap::detail(height.map_or(6.0, |height| {
        ((height - 2.0 - super::selection::line_height()) * 0.5).clamp(0.0, 6.0)
    }));
    let base = width.map_or_else(
        || LayoutStyle::flex_row().fill_width().grow(1.0),
        |w| LayoutStyle::flex_row().width(2.0f32.mul_add(-inset, w).max(0.0)),
    );
    let base = match height {
        Some(h) => base.height(2.0f32.mul_add(-inset, h).max(0.0)),
        None => base,
    };
    base.align_items(AlignItems::Center)
        .padding_x(space(3))
        .padding_y(padding_y)
}

fn pointer_up(event: &mut PointerEvent, interaction: InteractionState) {
    let _ = interaction.release_pointer(event);
}

fn displayed_text(
    value: Signal<String>,
    placeholder: TextContent,
    runtime: &TextFieldRuntime,
) -> Signal<String> {
    let preedit = runtime.input.ime.preedit;
    let caret = runtime.input.caret;
    let password = runtime.password;
    let reveal = runtime.reveal;
    derived(move || {
        let current = value.get();
        let composing = preedit.get();
        let hiding = super::mask::hiding(password, reveal);
        if !composing.is_empty() {
            let spliced =
                crate::widgets::text_edit::splice_preedit(&current, caret.get(), &composing);
            if hiding {
                super::mask::mask_chars(&spliced)
            } else {
                spliced
            }
        } else if current.is_empty() {
            text_content(&placeholder)
        } else if hiding {
            super::mask::mask_chars(&current)
        } else {
            current
        }
    })
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
) -> crate::style::StyleState {
    binary_style_state(interaction, disabled)
}
