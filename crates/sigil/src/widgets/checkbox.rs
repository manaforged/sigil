use std::rc::Rc;

use sigil_animation_runtime::presets;
use sigil_events::prelude::{Key, LogicalKey, NodeEventsExt};
use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::Signal;
use sigil_style::{
    ClassSource, NodeStyleExt, Size, StateHost, StateStyle, Style, StyleAccess, StyleAtom,
    StyleOverlay, StyleVariant, Theme, TransitionId,
};

use crate::interaction::InteractionState;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::shared::{
    BoolChange, BuilderMutator, activate_binary_key, activate_binary_pointer, binary_style_state,
    disabled_now, label, noop_bool_change, reactive_style_state,
};
use super::transition_seam::install_transition_animators;

const CHECKBOX_TRANSITION: TransitionId = presets::SMOOTH;
const CHECKBOX_COMPONENT_ID: u64 = 0xC4EC_B0CC;
const CHECKBOX_BOX_SIZE: f32 = 20.0;
const CHECKBOX_MARK_SIZE: f32 = 8.0;

/// Visual tone for [`checkbox`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CheckboxTone {
    /// Neutral surface checkbox.
    #[default]
    Neutral,
    /// Accent-coloured checked state.
    Primary,
}

/// Builder returned by [`checkbox`].
pub struct CheckboxBuilder {
    checked: Signal<bool>,
    label: Option<TextContent>,
    disabled: Option<Signal<bool>>,
    tone: CheckboxTone,
    on_change: Option<BoolChange>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

static EMPTY_BASE: &[StyleAtom] = &[];
// Root-row variant: every slot empty — the labelled row is transparent
// by default and stays that way. `stateful_style` uses it purely to
// publish the interaction slot cell on the row root so a user
// `.hover()` / `.pressed()` overlay resolves against the surface the
// pointer is actually over; the indicator child keeps the widget's own
// checked-state art (DS2 Phase 2 / T1 wiring).
static CHECKBOX_ROOT_VARIANT: StyleVariant = StyleVariant::new(EMPTY_BASE, StateStyle::EMPTY);
// Animator-only base slice: scanned by `install_transition_animators`
// for the `Transition` atom that flips it into the animated path. Does
// not affect the visual variant (which is still computed reactively).
static CHECKBOX_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(CHECKBOX_TRANSITION)];
static PRIMARY_CHECKED: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];
static PRIMARY_CHECKED_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent-hover")];
static PRIMARY_CHECKED_DOWN: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];
static NEUTRAL_CHECKED: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static NEUTRAL_CHECKED_DOWN: &[StyleAtom] =
    &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];
static UNCHECKED: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
// Hover skips straight to surface-pressed (rather than surface-hover)
// so the default→hover delta is perceptible on a 16-20px indicator.
// A border-darkening cue would be richer but the renderer has no
// border primitive (logged as a follow-up).
static UNCHECKED_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed")];
// Pressed uses the dedicated `surface-pressed-strong` token (a third
// step past `surface_pressed`) so the hover→pressed delta clears the
// same-hue JND at 16-20 px swatch scale. The previous 75 %-opacity
// blend over `surface_pressed` produced only a ~6 sRGB unit step vs
// hover — sub-perceptual on a 16 px indicator.
static UNCHECKED_DOWN: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];
// Disabled paints the dedicated `border` token at full opacity. The
// previous design used `surface-hover` + 50 % opacity, but the alpha
// blend composited back into the parent `bg-surface` (also near-white)
// and produced only an ~11 sRGB delta vs the resting state — below the
// JND on a 16-20 px indicator. `border` (alpha_mix(text, bg, 0.35) —
// ~(166,171,178) under the default light theme) carries a clear muted-
// grey reading that's visibly distinct from every other slot and works
// the same way under dark palettes (where it lifts toward the text
// colour instead). Opacity is dropped from the disabled slot — the
// W5.3 widget-catalog pass owns the richer "faded" semantic.
// Disabled paints the `surface-hover` token at full opacity. The
// previous design used `surface-hover` + 50 % opacity, but the alpha
// blend composited back over the parent `bg-surface` and produced
// only an ~11 sRGB delta versus the resting state. Dropping the
// opacity blend lets the full `surface-hover` brightness reach the
// framebuffer — that's a ~23 sRGB step versus rest and a ~21 step
// versus hover after the gamma encoding the offscreen RGBA8UNORM-SRGB
// target applies. `border` was the obvious alternative but its raw
// linear value (148,163,184) under the default theme renders close
// enough to `surface-pressed` after gamma to confuse the hover ↔
// disabled comparison. W5.3 owns the richer faded semantic.
static CHECKBOX_DISABLED: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static PRIMARY_MARK: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-accent")];
static NEUTRAL_MARK: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-surface")];
static NO_MARK: &[StyleAtom] = &[];

/// Create a checkbox widget.
#[must_use]
pub fn checkbox(checked: Signal<bool>) -> CheckboxBuilder {
    CheckboxBuilder::new(checked)
}

impl CheckboxBuilder {
    /// Create a checkbox builder.
    #[must_use]
    pub fn new(checked: Signal<bool>) -> Self {
        Self {
            checked,
            label: None,
            disabled: None,
            tone: CheckboxTone::Neutral,
            on_change: None,
            mutators: Vec::new(),
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Set the checkbox label.
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

    /// Set the visual tone.
    #[must_use]
    pub const fn tone(mut self, tone: CheckboxTone) -> Self {
        self.tone = tone;
        self
    }

    /// Run a callback after checked state changes.
    #[must_use]
    pub fn change(mut self, change: impl Fn(bool) + 'static) -> Self {
        self.on_change = Some(Rc::new(change));
        self
    }

    /// Add utility classes or atoms after the built-in checkbox defaults.
    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    /// Add struct-built style atoms after the built-in checkbox defaults.
    ///
    /// The mutable trait hook is [`crate::prelude::StyleAccess::style_mut`].
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.mutators.push(Box::new(move |node| node.style(style)));
        self
    }

    /// Attach a scoped theme to the checkbox root.
    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        let enabled = !disabled_now(self.disabled);
        let interaction = InteractionState::new(enabled);
        let on_change = self
            .on_change
            .as_ref()
            .map_or_else(noop_bool_change, Rc::clone);
        let layout_overrides = self.layout_overrides.clone();
        let has_label = self.label.is_some();
        // Drain user-supplied state-slot overlays before the base
        // accumulator attaches — they resolve against the row root's
        // interaction slot so `.hover(|s| s.bg(...))` paints the row.
        let user_overlay = self.style_accumulator.take_state_overlay();
        let mut node = self.base_node(interaction, Rc::clone(&on_change), user_overlay);
        for mutate in self.mutators {
            node = mutate(node);
        }
        // Styled accumulator installed after explicit mutators so the
        // trait surface layers ON TOP of `.class()` / `.style(Style)`
        // calls (additive composition per DR-049 §10).
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node.layout(layout_overrides.apply(checkbox_layout(has_label)))
    }

    fn base_node(
        &self,
        interaction: InteractionState,
        on_change: BoolChange,
        user_overlay: StyleOverlay,
    ) -> NodeBuilder {
        let checked = self.checked;
        let disabled = self.disabled;
        let tone = self.tone;
        let pointer_change = Rc::clone(&on_change);
        let enter_change = Rc::clone(&on_change);
        let space_change = on_change;
        let mut node = container()
            .role(Role::CheckBox)
            .label(accessible_label(self.label.as_ref()))
            .checked(checked)
            .layout(checkbox_layout(self.label.is_some()))
            .stateful_style(CHECKBOX_ROOT_VARIANT, user_overlay, move || {
                reactive_style_state(interaction, disabled)
            })
            .pointer_up(move |event| {
                activate_binary_pointer(event, interaction, disabled, checked, &pointer_change);
            })
            .key(Key::Enter, move |event| {
                activate_binary_key(event, interaction, disabled, checked, &enter_change);
            })
            .key(LogicalKey::character(" "), move |event| {
                activate_binary_key(event, interaction, disabled, checked, &space_change);
            })
            .child(checkbox_indicator(checked, disabled, tone, interaction));
        if let Some(label_source) = self.label.clone() {
            node = node.child(checkbox_label(label_source));
        }
        interaction.install(node)
    }
}

impl StyleAccess for CheckboxBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for CheckboxBuilder {}

impl WidgetLayoutExt for CheckboxBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for CheckboxBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn checkbox_layout(has_label: bool) -> LayoutStyle {
    let theme = Theme::default();
    let root_size = theme.control_height(Size::Default).px_or_zero();
    let gap = theme.gap(Size::Default).px_or_zero();
    let row = LayoutStyle::flex_row()
        .height(root_size)
        .gap(gap)
        .align_items(AlignItems::Center);
    if has_label {
        // Labeled checkbox flows with its parent — the row stretches to
        // fill available width and the label inside wraps naturally.
        row.fill_width()
    } else {
        // Unlabeled checkbox is just the indicator box.
        row.width(root_size)
    }
}

fn checkbox_indicator(
    checked: Signal<bool>,
    disabled: Option<Signal<bool>>,
    tone: CheckboxTone,
    interaction: InteractionState,
) -> NodeBuilder {
    let state_style = checkbox_state_style(tone, false);
    let node = container()
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(CHECKBOX_BOX_SIZE, CHECKBOX_BOX_SIZE)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .class("rounded-md")
        .class(CHECKBOX_ANIM_BASE)
        .style_reactive(move || {
            checkbox_variant(tone, checked.get()).state(binary_style_state(interaction, disabled))
        })
        .child(checkbox_mark(checked, tone));
    install_transition_animators(
        node,
        CHECKBOX_ANIM_BASE,
        state_style,
        CHECKBOX_COMPONENT_ID,
        interaction,
    )
}

const fn checkbox_state_style(tone: CheckboxTone, checked: bool) -> StateStyle {
    match (tone, checked) {
        (CheckboxTone::Primary, true) => StateStyle::new(
            PRIMARY_CHECKED,
            PRIMARY_CHECKED_HOVER,
            PRIMARY_CHECKED_HOVER,
            PRIMARY_CHECKED_DOWN,
            CHECKBOX_DISABLED,
        ),
        (CheckboxTone::Neutral, true) => StateStyle::new(
            NEUTRAL_CHECKED,
            NEUTRAL_CHECKED,
            NEUTRAL_CHECKED,
            NEUTRAL_CHECKED_DOWN,
            CHECKBOX_DISABLED,
        ),
        (_, false) => StateStyle::new(
            UNCHECKED,
            UNCHECKED_HOVER,
            UNCHECKED_HOVER,
            UNCHECKED_DOWN,
            CHECKBOX_DISABLED,
        ),
    }
}

fn checkbox_mark(checked: Signal<bool>, tone: CheckboxTone) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(CHECKBOX_MARK_SIZE, CHECKBOX_MARK_SIZE))
        .class("rounded-md")
        .style_reactive(move || checkbox_mark_atoms(tone, checked.get()))
}

fn checkbox_label(source: TextContent) -> impl IntoNode {
    // No fixed width — the label wraps to whatever the parent row
    // (`flex_row().fill_width()`) hands it.
    label(source).layout(LayoutStyle::block().fill_width())
}

fn accessible_label(label: Option<&TextContent>) -> String {
    label
        .and_then(TextContent::as_static_str)
        .map_or_else(|| String::from("Checkbox"), ToOwned::to_owned)
}

const fn checkbox_variant(tone: CheckboxTone, checked: bool) -> StyleVariant {
    match (tone, checked) {
        (CheckboxTone::Primary, true) => StyleVariant::new(
            EMPTY_BASE,
            StateStyle::new(
                PRIMARY_CHECKED,
                PRIMARY_CHECKED_HOVER,
                PRIMARY_CHECKED_HOVER,
                PRIMARY_CHECKED_DOWN,
                CHECKBOX_DISABLED,
            ),
        ),
        (CheckboxTone::Neutral, true) => StyleVariant::new(
            EMPTY_BASE,
            StateStyle::new(
                NEUTRAL_CHECKED,
                NEUTRAL_CHECKED,
                NEUTRAL_CHECKED,
                NEUTRAL_CHECKED_DOWN,
                CHECKBOX_DISABLED,
            ),
        ),
        (_, false) => StyleVariant::new(
            EMPTY_BASE,
            StateStyle::new(
                UNCHECKED,
                UNCHECKED_HOVER,
                UNCHECKED_HOVER,
                UNCHECKED_DOWN,
                CHECKBOX_DISABLED,
            ),
        ),
    }
}

const fn checkbox_mark_atoms(tone: CheckboxTone, checked: bool) -> &'static [StyleAtom] {
    match (tone, checked) {
        (CheckboxTone::Primary, true) => PRIMARY_MARK,
        (CheckboxTone::Neutral, true) => NEUTRAL_MARK,
        (_, false) => NO_MARK,
    }
}
