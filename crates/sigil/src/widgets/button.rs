use std::rc::Rc;

use reactive::Signal;
use sigil::widget_dev::*;

use crate::interaction::InteractionState;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::shared::{
    Activate, BuilderMutator, activate_key_reactive, activate_pointer_reactive, binary_style_state,
    disabled_now, noop_activate,
};
use super::transition_seam::install_transition_animators;

const BUTTON_TRANSITION: TransitionId = presets::SMOOTH;
const BUTTON_COMPONENT_ID: u64 = 0xB117_7000;

/// Visual tone for [`button`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonTone {
    /// Neutral surface button.
    #[default]
    Neutral,
    /// Accent-coloured primary action.
    Primary,
}

/// Builder returned by [`button`].
pub struct ButtonBuilder {
    label: String,
    tone: ButtonTone,
    width: f32,
    height: f32,
    enabled: bool,
    disabled: Option<Signal<bool>>,
    activate: Option<Activate>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

static PRIMARY_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("accent"),
    StyleAtom::Transition(BUTTON_TRANSITION),
];
static PRIMARY_REST: &[StyleAtom] = &[];
static PRIMARY_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent-hover")];
static PRIMARY_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent-hover")];
static PRIMARY_DOWN: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];
static NEUTRAL_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface"),
    StyleAtom::Transition(BUTTON_TRANSITION),
];
static NEUTRAL_REST: &[StyleAtom] = &[];
static NEUTRAL_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static NEUTRAL_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static NEUTRAL_DOWN: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed")];
static DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Opacity(5_000),
];

/// Create a button widget.
#[must_use]
pub fn button(label: impl Into<String>) -> ButtonBuilder {
    ButtonBuilder::new(label)
}

impl ButtonBuilder {
    /// Create a button builder.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        let theme = Theme::default();
        Self {
            label: label.into(),
            tone: ButtonTone::Neutral,
            width: theme.control_width(Size::Wide).px_or_zero(),
            height: theme.control_height(Size::Wide).px_or_zero(),
            enabled: true,
            disabled: None,
            activate: None,
            mutators: Vec::new(),
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Use the primary accent tone.
    #[must_use]
    pub const fn primary(mut self) -> Self {
        self.tone = ButtonTone::Primary;
        self
    }

    /// Use the neutral surface tone.
    #[must_use]
    pub const fn neutral(mut self) -> Self {
        self.tone = ButtonTone::Neutral;
        self
    }

    /// Set whether the button accepts focus and activation.
    #[must_use]
    pub const fn enabled(mut self, yes: bool) -> Self {
        self.enabled = yes;
        self
    }

    /// Bind a reactive disabled signal.
    ///
    /// Reactive parity with [`enabled`](Self::enabled): while `disabled`
    /// peeks `true` the button paints its disabled state and suppresses
    /// pointer / Enter activation. Flipping the signal updates the
    /// rendered state on the next reactive tick without rebuilding the
    /// tree.
    ///
    /// The constructor-time focusable / hittable bits still derive from
    /// [`enabled`](Self::enabled) and the signal's *initial* value —
    /// the reactive update covers the visual state and the activation
    /// guard, matching the [`CheckboxBuilder::disabled`] /
    /// [`SliderBuilder::disabled`] contract.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use sigil::widgets::button::button;
    /// use reactive::signal;
    ///
    /// let saving = signal(false);
    /// let node = button("Save").disabled(saving);
    /// // While `saving` is true the button greys out and won't activate.
    /// ```
    #[must_use]
    pub const fn disabled(mut self, disabled: Signal<bool>) -> Self {
        self.disabled = Some(disabled);
        self
    }

    /// Set a semantic theme-resolved control size.
    #[must_use]
    pub fn size(mut self, size: Size) -> Self {
        let theme = Theme::default();
        self.width = theme.control_width(size).px_or_zero();
        self.height = theme.control_height(size).px_or_zero();
        self
    }

    /// Set a fixed logical-pixel width.
    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Run a callback when the button is activated by pointer or Enter.
    #[must_use]
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    /// Add utility classes or atoms after the built-in button defaults.
    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    /// Add struct-built style atoms after the built-in button defaults.
    ///
    /// The mutable trait hook is [`crate::prelude::StyleAccess::style_mut`].
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.mutators.push(Box::new(move |node| node.style(style)));
        self
    }

    /// Attach a scoped theme to the button root.
    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        // Constructor enabled = static `enabled` AND the reactive
        // signal's initial value. Subsequent flips of the signal are
        // routed through `binary_style_state` (visual) and the
        // activate-guard wrappers (gesture).
        let enabled = self.enabled && !disabled_now(self.disabled);
        let interaction = InteractionState::new(enabled);
        let activate = self.activate.as_ref().map_or_else(noop_activate, Rc::clone);
        let layout_overrides = self.layout_overrides.clone();
        let mut node = self.base_node(interaction, Rc::clone(&activate));
        for mutate in self.mutators {
            node = mutate(node);
        }
        // Styled accumulator is installed AFTER the explicit mutators
        // so explicit `.class(...)` / `.style(Style)` calls land first
        // and the trait surface adds on top. Atoms are merged through
        // the existing `NodeStyleExt::style` path (a class-source push)
        // — same install seam as `container().styled()...`.
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node.layout(layout_overrides.apply(button_layout(self.width, self.height)))
    }

    fn base_node(&self, interaction: InteractionState, activate: Activate) -> NodeBuilder {
        let variant = button_variant(self.tone);
        let state_style = button_state_style(self.tone);
        let disabled = self.disabled;
        let pointer_activate = Rc::clone(&activate);
        let key_activate = activate;
        let node = container()
            .role(Role::Button)
            .label(self.label.clone())
            .layout(button_layout(self.width, self.height))
            .class("rounded-md")
            .class(variant.base())
            .style_reactive(move || {
                // Subscribe to the reactive disabled signal so flips
                // re-run this effect; `binary_style_state` only peeks.
                let _ = disabled.map(Signal::get);
                variant.state(binary_style_state(interaction, disabled))
            })
            .pointer_up(move |event| {
                activate_pointer_reactive(event, interaction, disabled, &pointer_activate);
            })
            .key(Key::Enter, move |event| {
                activate_key_reactive(event, interaction, disabled, &key_activate);
            })
            .child(self.label_node());
        let node = install_transition_animators(
            node,
            variant.base(),
            state_style,
            BUTTON_COMPONENT_ID,
            interaction,
        );
        interaction.install(node)
    }

    fn label_node(&self) -> TextBuilder {
        let theme = Theme::default();
        let label_pad_x = theme.padding(Size::Wide).px_or_zero();
        let width = 2.0_f32.mul_add(-label_pad_x, self.width).max(1.0);
        text(self.label.clone())
            .layout(LayoutStyle::block().width(width))
            .class(button_text_class(self.tone))
    }
}

impl StyleAccess for ButtonBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl WidgetLayoutExt for ButtonBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl StateHost for ButtonBuilder {}

impl IntoNode for ButtonBuilder {
    fn build(self, cx: &mut Cx) -> kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn button_layout(width: f32, height: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(width, height)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

const fn button_variant(tone: ButtonTone) -> StyleVariant {
    match tone {
        ButtonTone::Primary => StyleVariant::new(PRIMARY_BASE, button_state_style(tone)),
        ButtonTone::Neutral => StyleVariant::new(NEUTRAL_BASE, button_state_style(tone)),
    }
}

const fn button_state_style(tone: ButtonTone) -> StateStyle {
    match tone {
        ButtonTone::Primary => StateStyle::new(
            PRIMARY_REST,
            PRIMARY_FOCUS,
            PRIMARY_HOVER,
            PRIMARY_DOWN,
            DISABLED,
        ),
        ButtonTone::Neutral => StateStyle::new(
            NEUTRAL_REST,
            NEUTRAL_FOCUS,
            NEUTRAL_HOVER,
            NEUTRAL_DOWN,
            DISABLED,
        ),
    }
}

const fn button_text_class(tone: ButtonTone) -> &'static str {
    match tone {
        ButtonTone::Primary => "text-on-accent",
        ButtonTone::Neutral => "text-on-surface",
    }
}
