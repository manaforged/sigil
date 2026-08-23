use std::rc::Rc;

use crate::widget_dev::*;
use sigil_kernel::{AccessibleName, TextContent};
use sigil_reactive::Signal;
use sigil_style::{GHOST_VARIANT, Px, SOFT_VARIANT, SOLID_VARIANT, Tone};

use crate::interaction::InteractionState;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::shared::{
    Activate, BuilderMutator, activate_key_reactive, activate_pointer_reactive, disabled_now,
    noop_activate, or_disabled, reactive_style_state,
};
use super::transition_seam::install_transition_animators;

const BUTTON_TRANSITION: TransitionId = presets::SMOOTH;
const BUTTON_COMPONENT_ID: u64 = 0xB117_7000;

#[derive(Clone, Copy)]
enum ButtonWidth {
    Hug,
    Fixed(f32),
    Fill,
}

/// Static transition atom shared across all button variants — keeps
/// the SMOOTH spring on every state change so the visual is the same
/// whether the button is soft+neutral or solid+error.
static BUTTON_TRANSITION_ATOMS: &[StyleAtom] = &[StyleAtom::Transition(BUTTON_TRANSITION)];

/// 1px tonal edge at step 6. On soft fills it supplies the quiet
/// boundary (the Zed/ShadCN fill+border idiom); on solid fills it reads
/// as a slightly darker rim of the same hue.
static BUTTON_FRAME: &[StyleAtom] = &[StyleAtom::BorderWidth(Px(1)), StyleAtom::BorderToneStep(6)];

/// Label text-color atoms per variant. The label is a separate text
/// node so it must repeat the variant's text-color choice. `FgOnSolid`
/// reads each scale's WCAG-AA contrast color (works for every tone,
/// including warm/yellow); `FgToneStep(11)` is the step Radix uses
/// for muted text on soft-tinted backgrounds.
static LABEL_TEXT_SOLID: &[StyleAtom] = &[StyleAtom::FgOnSolid];
static LABEL_TEXT_SOFT: &[StyleAtom] = &[StyleAtom::FgToneStep(11)];

/// Builder returned by [`button`].
pub struct ButtonBuilder {
    label: TextContent,
    tone: Tone,
    variant: StyleVariant,
    /// Atoms applied to the label text node. Tracked alongside
    /// `variant` because the label is a separate text node — the
    /// variant's text-color atom sits on the container, not the label.
    label_atoms: &'static [StyleAtom],
    /// Whether the 1px tonal frame (BUTTON_FRAME) is painted. Ghost
    /// buttons drop it — a border at rest defeats the text-only look.
    frame: bool,
    width: ButtonWidth,
    height: f32,
    padding_x: f32,
    enabled: bool,
    disabled: Option<Signal<bool>>,
    activate: Option<Activate>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
    accessible: Option<AccessibleName>,
    pending: Option<Signal<bool>>,
}

/// Create a button widget.
#[must_use]
pub fn button(label: impl Into<TextContent>) -> ButtonBuilder {
    ButtonBuilder::new(label)
}

/// Ghost button whose visible label is a Unicode glyph.
///
/// Chain [`ButtonBuilder::accessible`] when the glyph is not a
/// readable word (`icon_button("▶", play).accessible("Play")`).
#[must_use]
pub fn icon_button(glyph: impl Into<TextContent>, activate: impl Fn() + 'static) -> ButtonBuilder {
    button(glyph).ghost().activate(activate)
}

impl ButtonBuilder {
    /// Create a button builder.
    ///
    /// Default look: soft variant + neutral tone (tinted surface
    /// background, readable on-surface text). Call [`primary`](Self::primary)
    /// to opt into the saturated accent CTA look, or
    /// [`tone`](Self::tone) / [`solid`](Self::solid) / [`soft`](Self::soft)
    /// for fine control.
    #[must_use]
    pub fn new(label: impl Into<TextContent>) -> Self {
        let theme = Theme::default();
        Self {
            label: label.into(),
            tone: Tone::Neutral,
            variant: SOFT_VARIANT,
            label_atoms: LABEL_TEXT_SOFT,
            frame: true,
            width: ButtonWidth::Hug,
            height: theme.control_height(Size::Default).px_or_zero(),
            padding_x: theme.padding(Size::Default).px_or_zero(),
            enabled: true,
            disabled: None,
            activate: None,
            mutators: Vec::new(),
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
            accessible: None,
            pending: None,
        }
    }

    /// Accessible name when the visible label is a glyph.
    #[must_use]
    pub fn accessible(mut self, name: impl Into<String>) -> Self {
        self.accessible = Some(AccessibleName::Static(name.into()));
        self
    }

    /// While true, the button paints disabled and ignores activate.
    /// Use for in-flight saves and network calls.
    #[must_use]
    pub const fn pending(mut self, pending: Signal<bool>) -> Self {
        self.pending = Some(pending);
        self
    }

    /// Sugar for [`tone(Tone::Primary)`](Self::tone) +
    /// [`solid`](Self::solid) — the classic saturated CTA button.
    #[must_use]
    pub const fn primary(mut self) -> Self {
        self.tone = Tone::Primary;
        self.variant = SOLID_VARIANT;
        self.label_atoms = LABEL_TEXT_SOLID;
        self
    }

    /// Sugar for [`tone(Tone::Neutral)`](Self::tone) +
    /// [`soft`](Self::soft) — the tinted surface default.
    #[must_use]
    pub const fn neutral(mut self) -> Self {
        self.tone = Tone::Neutral;
        self.variant = SOFT_VARIANT;
        self.label_atoms = LABEL_TEXT_SOFT;
        self
    }

    /// Sugar for [`tone(Tone::Error)`](Self::tone) +
    /// [`solid`](Self::solid) — the canonical destructive-action
    /// button (Delete, Discard, Revoke, etc.). Visually identical
    /// to `.solid().tone(Tone::Error)`; named for intent so call
    /// sites read as the design system pattern they are.
    #[must_use]
    pub const fn destructive(mut self) -> Self {
        self.tone = Tone::Error;
        self.variant = SOLID_VARIANT;
        self.label_atoms = LABEL_TEXT_SOLID;
        self
    }

    /// Sugar for [`tone(Tone::Neutral)`](Self::tone) + the ghost
    /// variant — text-only at rest, tinted hover. The row-action look
    /// (Edit / Delete inside table rows). Pair with
    /// [`tone(Tone::Error)`](Self::tone) for destructive row actions.
    #[must_use]
    pub const fn ghost(mut self) -> Self {
        self.tone = Tone::Neutral;
        self.variant = GHOST_VARIANT;
        self.label_atoms = LABEL_TEXT_SOFT;
        self.frame = false;
        self
    }

    /// Set the semantic tone (`Neutral`, `Primary`, `Info`, `Success`,
    /// `Warn`, `Error`). The widget's current variant determines HOW
    /// the tone paints — solid saturated fill vs. soft tinted fill.
    ///
    /// Adding a new tone is a two-line change to the [`Tone`] enum +
    /// [`Theme`](sigil_style::Theme) — no widget code changes.
    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Use the solid variant — saturated tone fill (step 9 bg, step 12
    /// text). The classic primary-button look across every tone.
    #[must_use]
    pub const fn solid(mut self) -> Self {
        self.variant = SOLID_VARIANT;
        self.label_atoms = LABEL_TEXT_SOLID;
        self
    }

    /// Use the soft variant — tinted tone background (step 3 bg,
    /// step 11 text). The secondary-action / chip-like look.
    #[must_use]
    pub const fn soft(mut self) -> Self {
        self.variant = SOFT_VARIANT;
        self.label_atoms = LABEL_TEXT_SOFT;
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
    /// guard, matching the [`crate::CheckboxBuilder::disabled`] /
    /// [`crate::SliderBuilder::disabled`] contract.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use sigil::widgets::button::button;
    /// use sigil_reactive::signal;
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
        if size == Size::Grow {
            self.width = ButtonWidth::Fill;
            self.height = theme.control_height(Size::Default).px_or_zero();
            self.padding_x = theme.padding(Size::Default).px_or_zero();
        } else {
            self.width = ButtonWidth::Hug;
            self.height = theme.control_height(size).px_or_zero();
            self.padding_x = theme.padding(size).px_or_zero();
        }
        self
    }

    /// Set a fixed logical-pixel width.
    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = ButtonWidth::Fixed(width);
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
    /// The mutable trait hook is [`sigil_style::StyleAccess::style_mut`].
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

    fn into_node_builder(mut self) -> NodeBuilder {
        self.disabled = or_disabled(self.disabled, self.pending);
        let enabled = self.enabled && !disabled_now(self.disabled);
        let interaction = InteractionState::new(enabled);
        let activate = self.activate.as_ref().map_or_else(noop_activate, Rc::clone);
        let layout_overrides = self.layout_overrides.clone();
        // Drain user-supplied state-slot overlays before the base
        // accumulator attaches — they get merged into the widget's
        // own StyleVariant so `.hover(|s| s.bg("error-9"))` etc.
        // actually paints (DS2 Phase 2 wiring).
        let user_overlay = self.style_accumulator.take_state_overlay();
        let mut node = self.base_node(interaction, Rc::clone(&activate), user_overlay);
        for mutate in self.mutators {
            node = mutate(node);
        }
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node.layout(layout_overrides.apply(button_layout(self.width, self.height, self.padding_x)))
    }

    fn base_node(
        &self,
        interaction: InteractionState,
        activate: Activate,
        user_overlay: sigil_style::StyleOverlay,
    ) -> NodeBuilder {
        // DS2.5 — `.stateful_style(...)` handles the slot-cell wiring
        // internally; `ToneContext` selects which scale the variant's
        // tone-step atoms resolve through. Adding a new tone is a
        // two-line change in `style` (enum + scale field) — zero
        // widget code touches.
        let variant = self.variant;
        let state_style = variant.states();
        let disabled = self.disabled;
        let pointer_activate = Rc::clone(&activate);
        let key_activate = activate;
        let name = self
            .accessible
            .clone()
            .unwrap_or_else(|| accessible_label(&self.label));
        let node = container()
            .role(Role::Button)
            .label(name)
            .layout(button_layout(self.width, self.height, self.padding_x))
            .class("rounded-md");
        let node = if self.frame {
            node.class(BUTTON_FRAME)
        } else {
            node
        };
        let node = node
            .class(variant.base())
            .class(BUTTON_TRANSITION_ATOMS)
            .tone(self.tone)
            .stateful_style(variant, user_overlay, move || {
                reactive_style_state(interaction, disabled)
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
        // Label paints with the same tone as the button container,
        // using the variant's preferred text atoms (FgOnSolid for
        // solid, FgToneStep(11) for soft).
        text(self.label.clone())
            .nowrap()
            .tone(self.tone)
            .class(self.label_atoms)
    }
}

fn accessible_label(label: &TextContent) -> AccessibleName {
    match label {
        TextContent::Static(value) => AccessibleName::Static(value.clone()),
        TextContent::Reactive(value) => AccessibleName::Derived(*value),
        TextContent::Compute(_) => AccessibleName::Static(label.resolve()),
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
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn button_layout(width: ButtonWidth, height: f32, padding_x: f32) -> LayoutStyle {
    let layout = LayoutStyle::flex_row()
        .height(height)
        .padding_x(padding_x)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center);
    match width {
        // Default flex-shrink is 1. The label still paints if the box
        // collapses, so hug/fixed buttons keep their intrinsic width.
        ButtonWidth::Hug => layout.shrink(0.0),
        ButtonWidth::Fixed(px) => layout.width(px).shrink(0.0),
        ButtonWidth::Fill => layout.fill_width(),
    }
}

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;
