use animation_runtime::presets;
use events::prelude::{NodeEventsExt, PointerEvent};
use kernel::{NodeBuilder, Role, TextBuilder, TextContent, container, fragment, text};
use layout::prelude::{LayoutBuilderExt, LayoutStyle};
use reactive::{Signal, derived_eq};
use style::{NodeStyleExt, StateStyle, StyleAtom, StyleVariant, TransitionId};

use crate::interaction::InteractionState;

use super::super::shared::binary_style_state;
use super::super::transition_seam::install_transition_animators;

const TEXT_EDIT_TRANSITION: TransitionId = presets::SMOOTH;
const TEXT_EDIT_COMPONENT_ID: u64 = 0x7E47_F100;
static TEXT_EDIT_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(TEXT_EDIT_TRANSITION)];

static EDIT_BASE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("border")];
static EDIT_REST: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("border")];
static EDIT_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-surface")];
static EDIT_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-surface")];
static EDIT_DOWN: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("on-surface"),
    StyleAtom::Opacity(8_800),
];
static EDIT_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("border"),
    StyleAtom::Opacity(5_000),
];

/// Cap on the number of physical line children we emit so a runaway
/// multi-megabyte paste doesn't blow up the tree on first paint.
const MAX_LINES: usize = 64;

/// Default outer width in logical pixels for the text-edit chrome.
const DEFAULT_WIDTH: f32 = 360.0;
/// Default outer height in logical pixels for the text-edit chrome.
const DEFAULT_HEIGHT: f32 = 160.0;
/// Border thickness in logical pixels — applied on every edge, so the
/// inner content area shrinks by `2 * BORDER_PX` along each axis.
const BORDER_PX: f32 = 1.0;
/// Inner content-area width: outer width minus a border on each side.
const INNER_WIDTH: f32 = DEFAULT_WIDTH - BORDER_PX - BORDER_PX;
/// Inner content-area height: outer height minus a border on each side.
const INNER_HEIGHT: f32 = DEFAULT_HEIGHT - BORDER_PX - BORDER_PX;

pub(super) fn build_base_node(
    value: Signal<String>,
    disabled: Option<Signal<bool>>,
    placeholder: &TextContent,
    interaction: InteractionState,
) -> NodeBuilder {
    let placeholder_str = text_content_untracked(placeholder);
    let displayed = displayed_text(value, interaction.focus_visible(), placeholder.clone());
    let state_style = edit_state_style();
    let node = container()
        .role(Role::TextInput)
        .label(accessible_label(placeholder_str.as_str()))
        .text_value(value)
        .placeholder(placeholder_str)
        .layout(edit_layout())
        .class("rounded-md")
        .class(TEXT_EDIT_ANIM_BASE)
        .style_reactive(move || edit_variant().state(edit_state(interaction, disabled)))
        .pointer_up(move |event| pointer_up(event, interaction))
        .child(edit_inner(displayed));
    install_transition_animators(
        node,
        TEXT_EDIT_ANIM_BASE,
        state_style,
        TEXT_EDIT_COMPONENT_ID,
        interaction,
    )
}

const fn edit_state_style() -> StateStyle {
    StateStyle::new(EDIT_REST, EDIT_FOCUS, EDIT_HOVER, EDIT_DOWN, EDIT_DISABLED)
}

pub(super) fn edit_layout() -> LayoutStyle {
    LayoutStyle::flex_column()
        .width(DEFAULT_WIDTH)
        .height(DEFAULT_HEIGHT)
}

fn edit_inner(displayed: Signal<String>) -> NodeBuilder {
    container()
        .layout(edit_inner_layout())
        .class("rounded-sm")
        .class("bg-surface")
        .child(edit_lines(displayed))
}

fn edit_inner_layout() -> LayoutStyle {
    LayoutStyle::flex_column()
        .width(INNER_WIDTH)
        .height(INNER_HEIGHT)
        .padding_x(12.0)
        .padding_y(8.0)
}

/// Render the buffer as one child text node per `\n`-delimited line.
///
/// Each line is a `text(line)` inside a `flex_column`. The line set is
/// installed via `children_reactive`: when the bound `displayed` signal
/// changes line count (e.g. typing `Enter`), the parent's child list
/// rebuilds against the fresh line vector. The text engine handles
/// intra-line shaping and (where supported) wrapping; line-aware caret
/// math is still followup work — see the module docs.
fn edit_lines(displayed: Signal<String>) -> NodeBuilder {
    let lines_signal = derived_eq(move || split_lines(displayed.get().as_str()));
    fragment()
        .layout(LayoutStyle::flex_column().width(334.0))
        .children_reactive(move || line_nodes(lines_signal.get()))
}

fn line_nodes(lines: Vec<String>) -> Vec<TextBuilder> {
    if lines.is_empty() {
        return vec![text(String::new()).class("text-on-surface")];
    }
    lines
        .into_iter()
        .take(MAX_LINES)
        .map(|line| {
            text(line)
                .layout(LayoutStyle::block().width(334.0))
                .class("text-on-surface")
        })
        .collect()
}

fn split_lines(source: &str) -> Vec<String> {
    if source.is_empty() {
        return Vec::new();
    }
    source.split('\n').map(ToOwned::to_owned).collect()
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
        .map_or_else(|| String::from("Text editor"), ToOwned::to_owned)
}

fn edit_state(interaction: InteractionState, disabled: Option<Signal<bool>>) -> style::StyleState {
    binary_style_state(interaction, disabled)
}

const fn edit_variant() -> StyleVariant {
    StyleVariant::new(
        EDIT_BASE,
        StateStyle::new(EDIT_REST, EDIT_FOCUS, EDIT_HOVER, EDIT_DOWN, EDIT_DISABLED),
    )
}
