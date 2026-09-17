use std::cell::Cell;
use std::rc::Rc;

use crate::animation_runtime::presets;
use crate::events::prelude::{Key, NodeEventsExt};
use crate::kernel::{AccessibleName, IntoNode, NodeBuilder, Role, container, text};
use crate::layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived_eq};
use crate::style::{NodeStyleExt, StateSlot, StateStyle, StyleAtom, StyleOverlay, StyleVariant};
use crate::widgets::interaction::InteractionState;
use crate::widgets::shared::{Activate, activate_key, activate_pointer};

use super::builder::{CloseHandler, TAB_HEIGHT};
use super::drag::{DragState, install_label_drag_hooks};

const TAB_PADDING_X: f32 = 12.0;
const CLOSE_AFFORDANCE_WIDTH: f32 = 16.0;

static TAB_BASE: &[StyleAtom] = &[StyleAtom::Transition(presets::SMOOTH)];
static TAB_REST: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static TAB_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static TAB_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static TAB_DOWN: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed")];
static TAB_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Opacity(5_000),
];

static TAB_ACTIVE_REST: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed")];
static TAB_ACTIVE_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];
static TAB_ACTIVE_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];
static TAB_ACTIVE_DOWN: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];

static TAB_VARIANT: StyleVariant = StyleVariant::new(
    TAB_BASE,
    StateStyle::new(TAB_REST, TAB_FOCUS, TAB_HOVER, TAB_DOWN, TAB_DISABLED),
);

static TAB_ACTIVE_VARIANT: StyleVariant = StyleVariant::new(
    TAB_BASE,
    StateStyle::new(
        TAB_ACTIVE_REST,
        TAB_ACTIVE_FOCUS,
        TAB_ACTIVE_HOVER,
        TAB_ACTIVE_DOWN,
        TAB_DISABLED,
    ),
);

fn tab_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .height(TAB_HEIGHT)
        .padding_x(TAB_PADDING_X)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn tab_label(label: String) -> impl IntoNode {
    text(label)
        .layout(LayoutStyle::block())
        .class("text-on-surface")
        .hittable(false)
}

fn close_button(index: usize, handler: Option<CloseHandler>) -> impl IntoNode {
    let interaction = InteractionState::new(true);
    let activate: Activate = Rc::new(move || {
        if let Some(cb) = handler.as_ref() {
            cb(index);
        }
    });
    let pointer_activate = Rc::clone(&activate);
    let node = interaction.install(
        container()
            .role(Role::Button)
            .label(AccessibleName::Static(String::from("Close tab")))
            .layout(LayoutStyle::block().fixed_size(CLOSE_AFFORDANCE_WIDTH, CLOSE_AFFORDANCE_WIDTH))
            .class("rounded-sm")
            .child(close_glyph()),
    );
    node.pointer_down(move |event| {
        event.stop_propagation();
    })
    .pointer_up(move |event| {
        event.stop_propagation();
        activate_pointer(event, interaction, &pointer_activate);
    })
}

fn close_glyph() -> impl IntoNode {
    text("\u{00D7}")
        .layout(LayoutStyle::block().width(CLOSE_AFFORDANCE_WIDTH))
        .class("text-on-surface")
}

#[allow(
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    reason = "the argument list mirrors the upstream API contract"
)]
pub(super) fn build_tab(
    index: usize,
    visual_index: usize,
    label: String,
    active: Signal<usize>,
    disabled: &Rc<Vec<usize>>,
    drag: Option<DragState>,
    closeable: bool,
    close_handler: Option<CloseHandler>,
    user_overlay: StyleOverlay,
) -> NodeBuilder {
    let is_disabled = disabled.contains(&index);
    let interaction = InteractionState::new(!is_disabled);
    let selected = derived_eq(move || active.get() == index);

    let slot = Rc::new(Cell::new(StateSlot::Rest));
    let style_slot = Rc::clone(&slot);

    let drag_for_activate = drag;
    let activate: Activate = Rc::new(move || {
        if is_disabled {
            return;
        }
        if drag_for_activate.is_some_and(DragState::consume_just_dragged) {
            return;
        }
        active.set(index);
    });
    let pointer_activate = Rc::clone(&activate);
    let key_activate = activate;

    let mut node = container()
        .role(Role::Tab)
        .label(label.clone())
        .checked(selected)
        .layout(tab_layout())
        .class("rounded-md")
        .class(TAB_BASE)
        .style_reactive(move || {
            let state = interaction.style_state();
            style_slot.set(state.slot());
            let variant = if selected.get() {
                TAB_ACTIVE_VARIANT
            } else {
                TAB_VARIANT
            };
            variant.state(state)
        })
        .state_overlay(user_overlay, slot)
        .pointer_up(move |event| activate_pointer(event, interaction, &pointer_activate))
        .key(Key::Enter, move |event| {
            activate_key(event, interaction, &key_activate);
        });

    if let Some(drag_state) = drag {
        node = install_label_drag_hooks(node, drag_state, visual_index);
    }

    node = node.child(tab_label(label));

    if closeable {
        node = node.child(close_button(index, close_handler.as_ref().map(Rc::clone)));
    }

    interaction.install(node)
}
