use std::cell::Cell;
use std::rc::Rc;

use crate::animation_runtime::presets;
use crate::events::NodeEventsExt;
use crate::kernel::{NodeBuilder, Role, TextContent, container};
use crate::layout::TextStyleCapture;
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived};
use crate::style::TransitionId;
use crate::style::{
    ColorAtom, ReactiveStateSlot, Rgba8, StateSlot, StyleAtom, StyleOverlay, space,
};

use crate::widgets::interaction::InteractionState;

use super::adorn::Adornments;
use super::handlers::TextFieldRuntime;

use super::super::shared::{FIELD_VARIANT, binary_style_state};

const TEXT_FIELD_TRANSITION: TransitionId = presets::SMOOTH;
static TEXT_FIELD_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(TEXT_FIELD_TRANSITION)];

static FIELD_INVALID: &[StyleAtom] = &[StyleAtom::BorderColor(ColorAtom::Rgba8(Rgba8 {
    r: 211,
    g: 47,
    b: 47,
    a: 255,
}))];

pub(super) struct FieldInputs {
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
    adorn: Adornments,
) -> NodeBuilder {
    let placeholder_str = placeholder.resolve();
    let displayed = displayed_text(&runtime, placeholder.clone());
    let slot_cell: ReactiveStateSlot = Rc::new(Cell::new(StateSlot::Rest));
    let disabled = runtime.input.disabled;
    let channel = field_style_channel(interaction, disabled, inputs.invalid, Rc::clone(&slot_cell));
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
            .text_value(runtime.input.value)
            .text_cursor(runtime.input.caret, runtime.input.selection);
    }
    let inner = container()
        .layout(field_inner_layout(&inputs, &adorn))
        .child(super::selection::text_region(
            runtime,
            interaction,
            displayed,
        ));
    let node = node
        .placeholder(placeholder_str)
        .layout(field_layout())
        .class(FIELD_VARIANT.base())
        .class(TEXT_FIELD_ANIM_BASE)
        .style_reactive(channel)
        .state_overlay(inputs.overlay, slot_cell)
        .seal_style()
        .pointer_up(move |event| {
            let _ = interaction.release_pointer(event);
        });
    adorn.wrap(node, inner)
}

fn field_style_channel(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    invalid: Option<Signal<bool>>,
    slot_cell: ReactiveStateSlot,
) -> impl Fn() -> &'static [StyleAtom] {
    move || {
        let state = binary_style_state(interaction, disabled);
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

fn field_inner_layout(inputs: &FieldInputs, adorn: &Adornments) -> LayoutStyle {
    let (width, height) = (inputs.width, inputs.height);
    let leading = adorn.leading.is_some();
    let trailing = adorn.trailing.is_some() || adorn.clear.is_some();
    let inset = crate::layout::snap::line(1.0);
    let padding_y = crate::layout::snap::detail(height.map_or(6.0, |height| {
        ((height - 2.0 - super::selection::line_height()) * 0.5).clamp(0.0, 6.0)
    }));
    let adorned = leading || trailing;
    let base = match width {
        _ if adorned => LayoutStyle::flex_row()
            .grow(1.0)
            .shrink(1.0)
            .min_size(0.0, crate::layout::Dimension::Auto),
        Some(w) => LayoutStyle::flex_row().width(2.0f32.mul_add(-inset, w).max(0.0)),
        None => LayoutStyle::flex_row().fill_width().grow(1.0),
    };
    let base = match height {
        Some(h) => base.height(2.0f32.mul_add(-inset, h).max(0.0)),
        None => base,
    };
    let edge = |side: bool| if side { space(2) } else { space(3) };
    base.align_items(AlignItems::Center)
        .padding_left(edge(leading))
        .padding_right(edge(trailing))
        .padding_y(padding_y)
}

fn displayed_text(runtime: &TextFieldRuntime, placeholder: TextContent) -> Signal<String> {
    let value = runtime.input.value;
    let preedit = runtime.input.ime.preedit;
    let caret = runtime.input.caret;
    let password = runtime.password;
    let reveal = runtime.reveal;
    derived(move || {
        let current = value.get();
        let composing = preedit.get();
        let hiding = super::mask::hiding(password, reveal);
        let shown = if !composing.is_empty() {
            crate::widgets::text_edit::splice_preedit(&current, caret.get(), &composing)
        } else if current.is_empty() {
            return placeholder.resolve();
        } else {
            current
        };
        if hiding {
            super::mask::mask_chars(&shown)
        } else {
            shown
        }
    })
}

fn accessible_label(placeholder: &str) -> String {
    if placeholder.is_empty() {
        String::from("Text field")
    } else {
        placeholder.to_owned()
    }
}
