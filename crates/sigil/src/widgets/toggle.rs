use std::rc::Rc;

use sigil_core::{Cx, IntoNode, NodeBuilder, Role, TextContent, container};
use sigil_events::prelude::{Key, LogicalKey, NodeEventsExt};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::Signal;
use sigil_style::{
    ClassSource, JustifyKind, NodeStyleExt, StateStyle, Style, StyleAtom, StyleVariant, Theme,
};

use crate::interaction::InteractionState;

use super::shared::{
    BoolChange, BuilderMutator, activate_binary_key, activate_binary_pointer, binary_style_state,
    disabled_now, label, noop_bool_change,
};

/// Builder returned by [`toggle`].
pub struct ToggleBuilder {
    on: Signal<bool>,
    label: Option<TextContent>,
    disabled: Option<Signal<bool>>,
    on_change: Option<BoolChange>,
    mutators: Vec<BuilderMutator>,
}

static EMPTY_BASE: &[StyleAtom] = &[];
static TOGGLE_ON: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("accent"),
    StyleAtom::JustifyContent(JustifyKind::End),
];
static TOGGLE_ON_HOVER: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("accent-hover"),
    StyleAtom::JustifyContent(JustifyKind::End),
];
static TOGGLE_ON_DOWN: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("accent-hover"),
    StyleAtom::JustifyContent(JustifyKind::End),
    StyleAtom::Opacity(8_800),
];
static TOGGLE_OFF: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::JustifyContent(JustifyKind::Start),
];
static TOGGLE_OFF_HOVER: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface"),
    StyleAtom::JustifyContent(JustifyKind::Start),
];
static TOGGLE_OFF_DOWN: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface"),
    StyleAtom::JustifyContent(JustifyKind::Start),
    StyleAtom::Opacity(8_800),
];
static TOGGLE_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Opacity(5_000),
];
static TOGGLE_THUMB: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-accent")];

/// Create a toggle widget.
#[must_use]
pub const fn toggle(on: Signal<bool>) -> ToggleBuilder {
    ToggleBuilder::new(on)
}

impl ToggleBuilder {
    /// Create a toggle builder.
    #[must_use]
    pub const fn new(on: Signal<bool>) -> Self {
        Self {
            on,
            label: None,
            disabled: None,
            on_change: None,
            mutators: Vec::new(),
        }
    }

    /// Set the toggle label.
    #[must_use]
    pub fn label(mut self, label: impl Into<TextContent>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set a reactive disabled signal.
    #[must_use]
    pub const fn disabled(mut self, disabled: Signal<bool>) -> Self {
        self.disabled = Some(disabled);
        self
    }

    /// Run a callback after state changes.
    #[must_use]
    pub fn on_change(mut self, change: impl Fn(bool) + 'static) -> Self {
        self.on_change = Some(Rc::new(change));
        self
    }

    /// Add utility classes or atoms after the built-in toggle defaults.
    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    /// Add struct-built style after the built-in toggle defaults.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.mutators.push(Box::new(move |node| node.style(style)));
        self
    }

    /// Attach a scoped theme to the toggle root.
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
        let mut node = self.base_node(interaction, Rc::clone(&on_change));
        for mutate in self.mutators {
            node = mutate(node);
        }
        node
    }

    fn base_node(&self, interaction: InteractionState, on_change: BoolChange) -> NodeBuilder {
        let on = self.on;
        let disabled = self.disabled;
        let pointer_change = Rc::clone(&on_change);
        let enter_change = Rc::clone(&on_change);
        let space_change = on_change;
        let mut node = container()
            .role(Role::Switch)
            .aria_label(accessible_label(self.label.as_ref()))
            .aria_checked_signal(on)
            .layout(toggle_layout(self.label.is_some()))
            .on_pointer_up(move |event| {
                activate_binary_pointer(event, interaction, disabled, on, &pointer_change);
            })
            .on_key(Key::Enter, move |event| {
                activate_binary_key(event, interaction, disabled, on, &enter_change);
            })
            .on_key(LogicalKey::character(" "), move |event| {
                activate_binary_key(event, interaction, disabled, on, &space_change);
            })
            .child(toggle_track(on, disabled, interaction));
        if let Some(label_source) = self.label.clone() {
            node = node.child(toggle_label(label_source));
        }
        interaction.install(node)
    }
}

impl IntoNode for ToggleBuilder {
    fn build(self, cx: &mut Cx) -> sigil_core::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn toggle_layout(has_label: bool) -> LayoutStyle {
    let width = if has_label { 184.0 } else { 52.0 };
    LayoutStyle::flex_row()
        .fixed_size(width, 32.0)
        .gap_px(8.0)
        .align_items(AlignItems::Center)
}

fn toggle_track(
    on: Signal<bool>,
    disabled: Option<Signal<bool>>,
    interaction: InteractionState,
) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(48.0, 28.0)
                .padding_px(3.0)
                .align_items(AlignItems::Center),
        )
        .class("rounded-md")
        .style_reactive(move || {
            toggle_variant(on.get()).state(binary_style_state(interaction, disabled))
        })
        .child(toggle_thumb())
}

fn toggle_thumb() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(22.0, 22.0))
        .class("rounded-md")
        .class(TOGGLE_THUMB)
}

fn toggle_label(source: TextContent) -> impl IntoNode {
    label(source).layout(LayoutStyle::block().width_px(124.0))
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
                TOGGLE_DISABLED,
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
                TOGGLE_DISABLED,
            ),
        )
    }
}
