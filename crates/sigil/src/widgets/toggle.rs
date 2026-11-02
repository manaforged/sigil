use crate::reactive::Reactive;
use crate::style::strings::StringKey;
use std::rc::Rc;

use crate::animation_runtime::presets;
use crate::kernel::{NodeBuilder, Role, TextContent, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, cols, rows};
use crate::reactive::{Signal, derived};
use crate::style::TransitionId;
use crate::style::{
    ControlSize, Metrics, StateStyle, Style, StyleAtom, StyleOverlay, StyleVariant, Token, space,
};
use crate::widgets::Widget;

use crate::widgets::interaction::InteractionState;

use super::layout_ext::LayoutOverrides;
use super::shared::{
    BoolChange, CONTROL_INSET, Press, RELEASE_ENTER_SPACE, accessible_name_or, binary_style_state,
    control_row, control_surface, disabled_now, noop_bool_change, press, static_label,
};

/// Builder of a switch.
pub struct ToggleBuilder {
    on: Signal<bool>,
    label: Option<TextContent>,
    accessible: Option<String>,
    disabled: Option<Signal<bool>>,
    on_change: Option<BoolChange>,
    layout_overrides: LayoutOverrides,
    style: Style,
}

builder_impls!([] ToggleBuilder: paint(style), widget, states(style), node);

static EMPTY_BASE: &[StyleAtom] = &[];
const TOGGLE_TRANSITION: TransitionId = presets::SMOOTH;
const CELL_TRACK_COLS: f32 = 4.0;
const CELL_TRACK_ON: &str = "[ ■]";
const CELL_TRACK_OFF: &str = "[■ ]";
static TOGGLE_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(TOGGLE_TRANSITION)];
static TOGGLE_ON: &[StyleAtom] = &[
    Token::Accent.bg(),
    StyleAtom::JustifyContent(JustifyContent::End),
];
static TOGGLE_ON_HOVER: &[StyleAtom] = &[
    Token::AccentHover.bg(),
    StyleAtom::JustifyContent(JustifyContent::End),
];
static TOGGLE_ON_DOWN: &[StyleAtom] = &[
    Token::AccentHover.bg(),
    StyleAtom::JustifyContent(JustifyContent::End),
    StyleAtom::Opacity(8_800),
];
static TOGGLE_OFF: &[StyleAtom] = &[
    Token::Gray6.bg(),
    StyleAtom::JustifyContent(JustifyContent::Start),
];
static TOGGLE_OFF_HOVER: &[StyleAtom] = &[
    Token::Gray7.bg(),
    StyleAtom::JustifyContent(JustifyContent::Start),
];
static TOGGLE_OFF_DOWN: &[StyleAtom] = &[
    Token::Gray7.bg(),
    StyleAtom::JustifyContent(JustifyContent::Start),
    StyleAtom::Opacity(8_800),
];
static TOGGLE_OFF_DISABLED: &[StyleAtom] = &[
    Token::Gray4.bg(),
    StyleAtom::JustifyContent(JustifyContent::Start),
];
static THUMB_ON: &[StyleAtom] = &[Token::OnAccent.bg()];
static THUMB_OFF: &[StyleAtom] = &[Token::Gray11.bg()];
static CELL_ON: &[StyleAtom] = &[Token::Accent.text()];
static CELL_OFF: &[StyleAtom] = &[Token::OnSurfaceMuted.text()];

/// An on/off switch.
#[must_use]
pub fn toggle(on: Signal<bool>) -> ToggleBuilder {
    ToggleBuilder::new(on)
}

impl ToggleBuilder {
    /// Creates a toggle builder.
    #[must_use]
    pub fn new(on: Signal<bool>) -> Self {
        Self {
            on,
            label: None,
            accessible: None,
            disabled: None,
            on_change: None,
            layout_overrides: LayoutOverrides::new(),
            style: Style::new(),
        }
    }

    /// Sets the accessible.
    #[must_use]
    pub fn accessible(mut self, name: impl Into<String>) -> Self {
        self.accessible = Some(name.into());
        self
    }

    /// Sets the label.
    #[must_use]
    pub fn label(mut self, label: impl Into<TextContent>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Sets the change.
    #[must_use]
    pub fn change(mut self, change: impl Fn(bool) + 'static) -> Self {
        self.on_change = Some(Rc::new(change));
        self
    }

    fn root(mut self) -> NodeBuilder {
        let enabled = !disabled_now(self.disabled);
        let interaction = InteractionState::new(enabled);
        let on_change = self.on_change.clone().unwrap_or_else(noop_bool_change);
        let has_label = self.label.is_some();
        let user_overlay = self.style.take_state_overlay();
        let mut node = self.base_node(interaction, on_change, user_overlay);
        if !self.style.is_empty() {
            node = node.style(self.style);
        }
        node.with(toggle_layout(has_label))
    }

    fn base_node(
        &self,
        interaction: InteractionState,
        on_change: BoolChange,
        user_overlay: StyleOverlay,
    ) -> NodeBuilder {
        let on = self.on;
        let disabled = self.disabled;
        let node = container()
            .role(Role::Switch)
            .label(accessible_name_or(
                self.accessible
                    .clone()
                    .or_else(|| static_label(self.label.as_ref()))
                    .map(TextContent::Static)
                    .as_ref(),
                StringKey::Toggle,
            ))
            .checked(on)
            .layout(toggle_layout(self.label.is_some()));
        let mut node = press(
            node,
            interaction,
            disabled,
            Press::Flip(on, on_change),
            RELEASE_ENTER_SPACE,
        )
        .child(if CellForm::new(false, true).pick() {
            cell_track(on)
        } else {
            toggle_track(on, disabled, interaction)
        });
        if let Some(label) = self.label.clone() {
            node = node.child(text(label).nowrap());
        }
        interaction.install(control_surface(node, interaction, disabled, user_overlay))
    }
}

fn toggle_layout(has_label: bool) -> LayoutStyle {
    let metrics = Metrics::current();
    let height = metrics.control_height(ControlSize::Default);
    let layout = control_row(LayoutStyle::flex_row())
        .height(height)
        .shrink(0.0)
        .gap(metrics.gap(ControlSize::Default))
        .align_items(AlignItems::Center);
    if has_label || CellForm::new(false, true).pick() {
        layout
    } else {
        layout.fixed_size(CONTROL_INSET.mul_add(2.0, space(9)), height)
    }
}

fn cell_track(on: Signal<bool>) -> NodeBuilder {
    container()
        .hittable(false)
        .layout(
            LayoutStyle::block()
                .width(cols(CELL_TRACK_COLS))
                .height(rows(1.0))
                .shrink(0.0),
        )
        .style_reactive(move || if on.get() { CELL_ON } else { CELL_OFF })
        .child(text(derived(move || {
            if on.get() {
                CELL_TRACK_ON
            } else {
                CELL_TRACK_OFF
            }
            .to_owned()
        })))
}

fn toggle_track(
    on: Signal<bool>,
    disabled: Option<Signal<bool>>,
    interaction: InteractionState,
) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(space(9), space(5))
                .padding(3.0)
                .align_items(AlignItems::Center),
        )
        .style(Style::new().radius(10))
        .class(TOGGLE_ANIM_BASE)
        .style_reactive(move || {
            toggle_variant(on.get()).state(binary_style_state(interaction, disabled))
        })
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(14.0, 14.0))
                .style(Style::new().radius(7))
                .class(TOGGLE_ANIM_BASE)
                .style_reactive(move || if on.get() { THUMB_ON } else { THUMB_OFF }),
        )
}

const fn toggle_variant(on: bool) -> StyleVariant {
    if on {
        StyleVariant::new(
            EMPTY_BASE,
            StateStyle::new(
                TOGGLE_ON,
                TOGGLE_ON_HOVER,
                TOGGLE_ON_HOVER,
                TOGGLE_ON_DOWN,
                TOGGLE_ON,
            ),
        )
    } else {
        StyleVariant::new(
            EMPTY_BASE,
            StateStyle::new(
                TOGGLE_OFF,
                TOGGLE_OFF_HOVER,
                TOGGLE_OFF_HOVER,
                TOGGLE_OFF_DOWN,
                TOGGLE_OFF_DISABLED,
            ),
        )
    }
}
