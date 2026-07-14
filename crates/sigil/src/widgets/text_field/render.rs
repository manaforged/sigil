use animation_runtime::presets;
use events::prelude::{NodeEventsExt, PointerEvent};
use kernel::{NodeBuilder, Role, TextContent, container};
use layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use reactive::{Signal, derived_eq};
use style::{ColorAtom, NodeStyleExt, Rgba8, StateStyle, StyleAtom, StyleVariant, TransitionId};

use crate::interaction::InteractionState;

use super::handlers::TextFieldRuntime;

use super::super::shared::binary_style_state;
use super::super::transition_seam::install_transition_animators;

const TEXT_FIELD_TRANSITION: TransitionId = presets::SMOOTH;
const TEXT_FIELD_COMPONENT_ID: u64 = 0x7E47_F000;
// Animator-only base slice scanned by `install_transition_animators`.
static TEXT_FIELD_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(TEXT_FIELD_TRANSITION)];

// Border colour for the rest/disabled states. `theme.border` resolves
// to slate step 7 (#CDCED7) which measures ~1.5:1 against the surface —
// well below the 3:1 WCAG 1.4.11 floor for non-text UI boundaries
// (visual review 2026-05-21). `gray-9` (#8B8D98 in slate-light)
// measures ~3.3:1 and lifts the field edge to a compliant boundary
// without going so dark it reads as a focused-state border.
static FIELD_BASE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("gray-9")];
static FIELD_REST: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("gray-9")];
static FIELD_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-surface")];
static FIELD_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-surface")];
static FIELD_DOWN: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("on-surface"),
    StyleAtom::Opacity(8_800),
];
static FIELD_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("gray-9"),
    StyleAtom::Opacity(5_000),
];
// Invalid-state overlay: a saturated red border paint that overrides
// every other state slot when the `invalid_when` signal peeks `true`.
// Colour matches WCAG AAA-compliant Material "error" (#D32F2F → sRGB
// (211,47,47)) so the 1 px-thick outer band stays legible against the
// default light + dark surfaces.
pub(super) static FIELD_INVALID: &[StyleAtom] =
    &[StyleAtom::BackgroundColor(ColorAtom::Rgba8(Rgba8 {
        r: 211,
        g: 47,
        b: 47,
        a: 255,
    }))];

/// The reactive signal inputs a text field binds — grouped so the
/// render entry point stays within the 5-argument hygiene cap.
#[derive(Clone, Copy)]
pub(super) struct FieldInputs {
    pub(super) value: Signal<String>,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) invalid: Option<Signal<bool>>,
}

pub(super) fn build_base_node(
    inputs: FieldInputs,
    placeholder: &TextContent,
    interaction: InteractionState,
    width: Option<f32>,
    runtime: TextFieldRuntime,
) -> NodeBuilder {
    let FieldInputs {
        value,
        disabled,
        invalid,
    } = inputs;
    let placeholder_str = text_content_untracked(placeholder);
    let displayed = displayed_text(
        value,
        interaction.focus_visible(),
        placeholder.clone(),
        runtime.ime.preedit,
        runtime.caret,
    );
    let state_style = field_state_style();
    let node = container()
        .role(Role::TextInput)
        .label(accessible_label(placeholder_str.as_str()))
        .text_value(value)
        .placeholder(placeholder_str)
        .layout(field_layout())
        .class("rounded-md")
        .class(TEXT_FIELD_ANIM_BASE)
        .style_reactive(move || {
            // Invalid overrides every other state slot — a red border
            // is the universal "fix this before continuing" cue. We
            // call `.get()` (not `.peek()`) so the effect re-subscribes
            // whenever the signal flips.
            if invalid.is_some_and(Signal::get) {
                FIELD_INVALID
            } else {
                field_variant().state(field_state(interaction, disabled))
            }
        })
        .pointer_up(move |event| pointer_up(event, interaction))
        .child(field_inner(runtime, interaction, displayed, width));
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

pub(super) fn field_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_width()
        .align_items(AlignItems::Center)
        .padding(1.0)
}

fn field_inner(
    runtime: TextFieldRuntime,
    interaction: InteractionState,
    displayed: Signal<String>,
    width: Option<f32>,
) -> NodeBuilder {
    container()
        .layout(field_inner_layout(width))
        .class("rounded-sm")
        .class("bg-surface")
        .child(super::selection::text_region(
            runtime,
            interaction,
            displayed,
            width,
        ))
}

fn field_inner_layout(width: Option<f32>) -> LayoutStyle {
    // An explicit builder width pins the inner track to width − 2 (the
    // 1 px outer padding band each side). With no override the inner
    // fills whatever the outer border frame gives it — the old fixed
    // `width(178)` default produced a "gray right cap" in any host
    // wider than 180 px, exposing a strip of the outer's border-coloured
    // background on the right (visual review 2026-05-21).
    let base = width.map_or_else(
        || LayoutStyle::flex_row().fill_width().grow(1.0),
        |w| LayoutStyle::flex_row().width((w - 2.0).max(0.0)),
    );
    base.align_items(AlignItems::Center)
        .padding_x(12.0)
        .padding_y(8.0)
}

fn pointer_up(event: &mut PointerEvent, interaction: InteractionState) {
    let _ = interaction.release_pointer(event);
}

/// The field's displayed text: the committed buffer with any in-flight
/// IME preedit spliced in at the caret for display only (`value` never
/// carries preedit text — DR-051 §Decision-3's `Commit` handler is the
/// only writer of the committed buffer).
fn displayed_text(
    value: Signal<String>,
    focused: Signal<bool>,
    placeholder: TextContent,
    preedit: Signal<String>,
    caret: Signal<usize>,
) -> Signal<String> {
    derived_eq(move || {
        let current = value.get();
        let composing = preedit.get();
        if !composing.is_empty() {
            splice_preedit(&current, caret.get(), &composing)
        } else if current.is_empty() && !focused.get() {
            text_content(&placeholder)
        } else {
            current
        }
    })
}

/// Insert `preedit` into `value` at the `caret` byte offset, for display
/// only. The caret mirrors `TextEdit`'s caret, but value and caret publish
/// through separate signals — the clamp guards the transient state between
/// the two writes (same fix as `text_edit`'s gated HIGH-1).
fn splice_preedit(value: &str, caret: usize, preedit: &str) -> String {
    let caret = crate::widgets::text_edit::floor_char_boundary(value, caret.min(value.len()));
    let mut out = String::with_capacity(value.len() + preedit.len());
    out.push_str(&value[..caret]);
    out.push_str(preedit);
    out.push_str(&value[caret..]);
    out
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

fn field_state(interaction: InteractionState, disabled: Option<Signal<bool>>) -> style::StyleState {
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
