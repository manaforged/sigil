use std::rc::Rc;

use sigil_events::prelude::{Key, LogicalKey, NodeEventsExt};
use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::Signal;
use sigil_style::{
    ClassSource, NodeStyleExt, StateHost, StateStyle, Style, StyleAccess, StyleAtom, StyleVariant,
    Theme,
};

use crate::interaction::InteractionState;

use super::shared::{
    BoolChange, BuilderMutator, activate_binary_key, activate_binary_pointer, binary_style_state,
    disabled_now, label, noop_bool_change,
};

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
}

static EMPTY_BASE: &[StyleAtom] = &[];
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
// Disabled uses surface-hover (the lighter of the two muted tones)
// PLUS 50% opacity. The opacity layer is what differentiates disabled
// from hover — hover lands on surface-pressed at full opacity, so a
// disabled swatch reads as "faded surface-hover" rather than "deeper
// surface". Reactive state applies LAST in the cascade, beating any
// inline `bg-surface` class. A border primitive would carry a richer
// cue but the renderer has none; opacity + distinct fill is the only
// pair of axes available at 16-20px.
static CHECKBOX_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Opacity(5_000),
];
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
    pub fn on_change(mut self, change: impl Fn(bool) + 'static) -> Self {
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

    /// Add struct-built style after the built-in checkbox defaults.
    ///
    /// Coexists with the `Styled::style(&mut self) -> &mut Style` trait
    /// method — signatures disambiguate by receiver + arity (DR-049 §11).
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
        // Styled accumulator installed after explicit mutators so the
        // trait surface layers ON TOP of `.class()` / `.style(Style)`
        // calls (additive composition per DR-049 §10).
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node
    }

    fn base_node(&self, interaction: InteractionState, on_change: BoolChange) -> NodeBuilder {
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
            .on_pointer_up(move |event| {
                activate_binary_pointer(event, interaction, disabled, checked, &pointer_change);
            })
            .on_key(Key::Enter, move |event| {
                activate_binary_key(event, interaction, disabled, checked, &enter_change);
            })
            .on_key(LogicalKey::character(" "), move |event| {
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
    fn style(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for CheckboxBuilder {}

impl IntoNode for CheckboxBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn checkbox_layout(has_label: bool) -> LayoutStyle {
    let width = if has_label { 180.0 } else { 32.0 };
    LayoutStyle::flex_row()
        .fixed_size(width, 32.0)
        .gap_px(8.0)
        .align_items(AlignItems::Center)
}

fn checkbox_indicator(
    checked: Signal<bool>,
    disabled: Option<Signal<bool>>,
    tone: CheckboxTone,
    interaction: InteractionState,
) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(20.0, 20.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .class("rounded-md")
        .style_reactive(move || {
            checkbox_variant(tone, checked.get()).state(binary_style_state(interaction, disabled))
        })
        .child(checkbox_mark(checked, tone))
}

fn checkbox_mark(checked: Signal<bool>, tone: CheckboxTone) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(8.0, 8.0))
        .class("rounded-md")
        .style_reactive(move || checkbox_mark_atoms(tone, checked.get()))
}

fn checkbox_label(source: TextContent) -> impl IntoNode {
    label(source).layout(LayoutStyle::block().width_px(144.0))
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
