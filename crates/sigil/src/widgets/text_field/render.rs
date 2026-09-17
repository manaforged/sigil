use std::cell::Cell;
use std::rc::Rc;

use crate::animation_runtime::presets;
use crate::events::prelude::{NodeEventsExt, PointerEvent};
use crate::kernel::{NodeBuilder, Role, TextContent, container};
use crate::layout::TextStyleCapture;
use crate::layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived_eq};
use crate::style::{
    ColorAtom, NodeStyleExt, Px, ReactiveStateSlot, Rgba8, StateSlot, StateStyle, StyleAtom,
    StyleOverlay, StyleVariant, TransitionId,
};

use crate::widgets::interaction::InteractionState;

use super::handlers::TextFieldRuntime;

use super::super::shared::binary_style_state;

const TEXT_FIELD_TRANSITION: TransitionId = presets::SMOOTH;
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
    let displayed = displayed_text(value, placeholder.clone(), &runtime);
    let slot_cell: ReactiveStateSlot = Rc::new(Cell::new(StateSlot::Rest));
    let channel = field_style_channel(interaction, disabled, invalid, Rc::clone(&slot_cell));
    let text_style = runtime.text_style;
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
        node = node.text_value(value);
    }
    node.placeholder(placeholder_str)
        .layout(field_layout())
        .class("rounded-md")
        .class(FIELD_BASE)
        .class(TEXT_FIELD_ANIM_BASE)
        .style_reactive(channel)
        .state_overlay(overlay, slot_cell)
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
            field_variant().state(state)
        }
    }
}

pub(super) fn field_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_width()
        .min_size(0.0, crate::layout::prelude::Dimension::Auto)
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
    runtime: &TextFieldRuntime,
) -> Signal<String> {
    let preedit = runtime.ime.preedit;
    let caret = runtime.caret;
    let password = runtime.password;
    let reveal = runtime.reveal;
    derived_eq(move || {
        let current = value.get();
        let composing = preedit.get();
        let hiding = super::mask::hiding(password, reveal);
        if !composing.is_empty() {
            let spliced = splice_preedit(&current, caret.get(), &composing);
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

fn splice_preedit(value: &str, caret: usize, preedit: &str) -> String {
    let caret = crate::widgets::text_edit::floor_char_boundary(value, caret.min(value.len()));
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
) -> crate::style::StyleState {
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
