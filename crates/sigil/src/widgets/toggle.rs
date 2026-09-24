use std::rc::Rc;

use crate::animation_runtime::presets;
use crate::events::prelude::{Key, LogicalKey, NodeEventsExt};
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container};
use crate::layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{
    ClassSource, JustifyKind, NodeStyleExt, StateStyle, Style, StyleAtom, StyleOverlay,
    StyleVariant, Theme, Token, TransitionId, space,
};

use crate::widgets::interaction::InteractionState;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::shared::{
    BoolChange, BuilderMutator, CONTROL_INSET, activate_binary_key, activate_binary_pointer,
    binary_style_state, control_row, control_surface, disabled_now, label, noop_bool_change,
};

const TOGGLE_TRANSITION: TransitionId = presets::SMOOTH;

pub struct ToggleBuilder {
    on: Signal<bool>,
    label: Option<TextContent>,
    disabled: Option<Signal<bool>>,
    on_change: Option<BoolChange>,
    mutators: Vec<BuilderMutator>,
    layout_overrides: LayoutOverrides,
}

impl WidgetLayoutExt for ToggleBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

static EMPTY_BASE: &[StyleAtom] = &[];
static TOGGLE_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(TOGGLE_TRANSITION)];
static TOGGLE_ON: &[StyleAtom] = &[
    Token::Accent.bg(),
    StyleAtom::JustifyContent(JustifyKind::End),
];
static TOGGLE_ON_HOVER: &[StyleAtom] = &[
    Token::AccentHover.bg(),
    StyleAtom::JustifyContent(JustifyKind::End),
];
static TOGGLE_ON_DOWN: &[StyleAtom] = &[
    Token::AccentHover.bg(),
    StyleAtom::JustifyContent(JustifyKind::End),
    StyleAtom::Opacity(8_800),
];
static TOGGLE_OFF: &[StyleAtom] = &[
    Token::SurfaceHover.bg(),
    StyleAtom::JustifyContent(JustifyKind::Start),
];
static TOGGLE_OFF_HOVER: &[StyleAtom] = &[
    Token::Surface.bg(),
    StyleAtom::JustifyContent(JustifyKind::Start),
];
static TOGGLE_OFF_DOWN: &[StyleAtom] = &[
    Token::Surface.bg(),
    StyleAtom::JustifyContent(JustifyKind::Start),
    StyleAtom::Opacity(8_800),
];
static TOGGLE_ON_DISABLED: &[StyleAtom] = &[
    Token::Accent.bg(),
    StyleAtom::JustifyContent(JustifyKind::End),
];
static TOGGLE_OFF_DISABLED: &[StyleAtom] = &[
    Token::SurfaceHover.bg(),
    StyleAtom::JustifyContent(JustifyKind::Start),
];
static TOGGLE_THUMB: &[StyleAtom] = &[Token::OnAccent.bg()];

#[must_use]
pub const fn toggle(on: Signal<bool>) -> ToggleBuilder {
    ToggleBuilder::new(on)
}

impl ToggleBuilder {
    #[must_use]
    pub const fn new(on: Signal<bool>) -> Self {
        Self {
            on,
            label: None,
            disabled: None,
            on_change: None,
            mutators: Vec::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<TextContent>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub const fn disabled(mut self, disabled: Signal<bool>) -> Self {
        self.disabled = Some(disabled);
        self
    }

    #[must_use]
    pub fn change(mut self, change: impl Fn(bool) + 'static) -> Self {
        self.on_change = Some(Rc::new(change));
        self
    }

    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.mutators.push(Box::new(move |node| node.style(style)));
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let enabled = !disabled_now(self.disabled);
        let interaction = InteractionState::new(enabled);
        let on_change = self
            .on_change
            .as_ref()
            .map_or_else(noop_bool_change, Rc::clone);
        let layout_overrides = self.layout_overrides.clone();
        let has_label = self.label.is_some();
        let mut node = self.base_node(interaction, Rc::clone(&on_change));
        for mutate in self.mutators {
            node = mutate(node);
        }
        node.layout(layout_overrides.apply(toggle_layout(has_label)))
    }

    fn base_node(&self, interaction: InteractionState, on_change: BoolChange) -> NodeBuilder {
        let on = self.on;
        let disabled = self.disabled;
        let pointer_change = Rc::clone(&on_change);
        let enter_change = Rc::clone(&on_change);
        let space_change = on_change;
        let mut node = container()
            .role(Role::Switch)
            .label(accessible_label(self.label.as_ref()))
            .checked(on)
            .layout(toggle_layout(self.label.is_some()))
            .pointer_up(move |event| {
                activate_binary_pointer(event, interaction, disabled, on, &pointer_change);
            })
            .key(Key::Enter, move |event| {
                activate_binary_key(event, interaction, disabled, on, &enter_change);
            })
            .key(LogicalKey::character(" "), move |event| {
                activate_binary_key(event, interaction, disabled, on, &space_change);
            })
            .child(toggle_track(on, disabled, interaction));
        if let Some(label_source) = self.label.clone() {
            node = node.child(toggle_label(label_source));
        }
        interaction.install(control_surface(
            node,
            interaction,
            disabled,
            StyleOverlay::default(),
        ))
    }
}

impl IntoNode for ToggleBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn toggle_layout(has_label: bool) -> LayoutStyle {
    let layout = control_row(LayoutStyle::flex_row())
        .height(space(7))
        .shrink(0.0)
        .gap(space(2))
        .align_items(AlignItems::Center);
    if has_label {
        layout
    } else {
        layout.fixed_size(CONTROL_INSET.mul_add(2.0, space(9)), space(7))
    }
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
        .child(toggle_thumb())
}

fn toggle_thumb() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(14.0, 14.0))
        .style(Style::new().radius(7))
        .class(TOGGLE_THUMB)
}

fn toggle_label(source: TextContent) -> impl IntoNode {
    label(source)
}

fn accessible_label(label: Option<&TextContent>) -> String {
    label
        .and_then(TextContent::as_static_str)
        .map_or_else(|| String::from("Toggle"), ToOwned::to_owned)
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
                TOGGLE_ON_DISABLED,
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
