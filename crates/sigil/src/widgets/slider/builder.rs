use std::ops::RangeInclusive;
use std::rc::Rc;

use animation_runtime::presets;
use events::prelude::{NodeEventsExt, PointerEvent};
use kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use reactive::Signal;
use style::{
    ClassSource, NodeStyleExt, StateHost, StateStyle, Style, StyleAccess, StyleAtom, StyleVariant,
    Theme, TransitionId,
};

use crate::interaction::InteractionState;

use super::super::shared::{BuilderMutator, binary_style_state, disabled_now};
use super::super::transition_seam::install_transition_animators;
use super::handlers::{
    attach_key_handlers, normalized_step, pointer_down, pointer_move, pointer_up, quantize,
};

const SLIDER_TRANSITION: TransitionId = presets::SMOOTH;
const SLIDER_COMPONENT_ID: u64 = 0x5110_E000;
// Animator-only base slice: scanned by `install_transition_animators`
// for the `Transition` atom that flips the thumb into the animated path.
static SLIDER_THUMB_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(SLIDER_TRANSITION)];

pub(super) type ValueChange = Rc<dyn Fn(f32) + 'static>;

const HORIZONTAL_WIDTH: f32 = 180.0;
const HORIZONTAL_HEIGHT: f32 = 28.0;
const VERTICAL_WIDTH: f32 = 28.0;
const VERTICAL_HEIGHT: f32 = 180.0;
const THUMB_SIZE: f32 = 22.0;

static TRACK: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static TRACK_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Opacity(5_000),
];
static THUMB_REST: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];
static THUMB_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent-hover")];
static THUMB_DOWN: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("accent-hover"),
    StyleAtom::Opacity(8_800),
];
static THUMB_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface"),
    StyleAtom::Opacity(5_000),
];

/// Slider orientation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Orientation {
    /// Horizontal left-to-right slider.
    #[default]
    Horizontal,
    /// Vertical bottom-to-top slider.
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SliderRange {
    pub(super) min: f32,
    pub(super) max: f32,
}

#[derive(Clone, Copy)]
pub(super) struct SliderRuntime {
    pub(super) value: Signal<f32>,
    pub(super) range: SliderRange,
    pub(super) step: f32,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) orientation: Orientation,
}

/// Builder returned by [`slider`].
pub struct SliderBuilder {
    value: Signal<f32>,
    range: SliderRange,
    step: f32,
    disabled: Option<Signal<bool>>,
    on_change: Option<ValueChange>,
    orientation: Orientation,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
}

/// Create a slider widget.
#[must_use]
pub fn slider(value: Signal<f32>, range: RangeInclusive<f32>) -> SliderBuilder {
    SliderBuilder::new(value, range)
}

impl SliderBuilder {
    /// Create a slider builder.
    #[must_use]
    pub fn new(value: Signal<f32>, range: RangeInclusive<f32>) -> Self {
        let start = *range.start();
        let end = *range.end();
        Self {
            value,
            range: SliderRange::from_bounds(start, end),
            step: 1.0,
            disabled: None,
            on_change: None,
            orientation: Orientation::Horizontal,
            mutators: Vec::new(),
            style_accumulator: Style::new(),
        }
    }

    /// Set the step used for pointer quantization and keyboard movement.
    #[must_use]
    pub const fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Set a reactive disabled signal.
    #[must_use]
    pub const fn disabled(mut self, disabled: Signal<bool>) -> Self {
        self.disabled = Some(disabled);
        self
    }

    /// Run a callback when a user gesture commits a value.
    #[must_use]
    pub fn change(mut self, change: impl Fn(f32) + 'static) -> Self {
        self.on_change = Some(Rc::new(change));
        self
    }

    /// Set the slider orientation.
    #[must_use]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Add utility classes or atoms after the built-in slider defaults.
    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    /// Add struct-built style atoms after the built-in slider defaults.
    ///
    /// The mutable trait hook is [`crate::prelude::StyleAccess::style_mut`].
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.mutators.push(Box::new(move |node| node.style(style)));
        self
    }

    /// Attach a scoped theme to the slider root.
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
            .map_or_else(noop_value_change, Rc::clone);
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
}

impl StyleAccess for SliderBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for SliderBuilder {}

impl SliderBuilder {
    fn base_node(&self, interaction: InteractionState, on_change: ValueChange) -> NodeBuilder {
        let runtime = SliderRuntime::from_builder(self);
        let node = slider_root(runtime, interaction)
            .pointer_down(move |event| pointer_down(event, interaction, runtime))
            .pointer_move(move |event| pointer_move(event, interaction, runtime))
            .pointer_up(move |event| {
                pointer_up(event, interaction, runtime.value, &on_change);
            });
        interaction.install(attach_key_handlers(node, interaction, runtime))
    }
}

impl IntoNode for SliderBuilder {
    fn build(self, cx: &mut Cx) -> kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

impl SliderRange {
    pub(super) const fn from_bounds(start: f32, end: f32) -> Self {
        Self {
            min: start.min(end),
            max: start.max(end),
        }
    }

    pub(super) fn span(self) -> f32 {
        (self.max - self.min).max(f32::EPSILON)
    }

    pub(super) const fn clamp(self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }
}

impl SliderRuntime {
    pub(super) const fn from_builder(builder: &SliderBuilder) -> Self {
        Self {
            value: builder.value,
            range: builder.range,
            step: builder.step,
            disabled: builder.disabled,
            orientation: builder.orientation,
        }
    }

    pub(super) fn disabled(self) -> bool {
        disabled_now(self.disabled)
    }

    pub(super) fn set_from_position(self, event: &PointerEvent) {
        let ratio = match self.orientation {
            Orientation::Horizontal => event.position.x / HORIZONTAL_WIDTH,
            Orientation::Vertical => 1.0 - (event.position.y / VERTICAL_HEIGHT),
        };
        let raw = self
            .range
            .span()
            .mul_add(ratio.clamp(0.0, 1.0), self.range.min);
        self.value.set(quantize(self.range, raw, self.step));
    }

    pub(super) fn step_value(self, increase: bool) {
        let delta = normalized_step(self.step);
        let signed = if increase { delta } else { -delta };
        self.value
            .set(quantize(self.range, self.value.peek() + signed, self.step));
    }

    pub(super) fn page_step_value(self, increase: bool) {
        let delta = normalized_step(self.step) * 10.0;
        let signed = if increase { delta } else { -delta };
        self.value
            .set(quantize(self.range, self.value.peek() + signed, self.step));
    }

    pub(super) fn jump_to_end(self, max: bool) {
        let target = if max { self.range.max } else { self.range.min };
        self.value.set(target);
    }
}

fn slider_root(runtime: SliderRuntime, interaction: InteractionState) -> NodeBuilder {
    container()
        .role(Role::Slider)
        .label("Slider")
        .value(runtime.value, runtime.range.min, runtime.range.max)
        .layout(slider_layout(runtime.orientation))
        .class("rounded-md")
        .class(TRACK)
        .style_reactive(move || {
            slider_track_variant().state(binary_style_state(interaction, runtime.disabled))
        })
        .child(slider_thumb(interaction, runtime.disabled))
}

fn slider_layout(orientation: Orientation) -> LayoutStyle {
    match orientation {
        Orientation::Horizontal => LayoutStyle::flex_row()
            .fixed_size(HORIZONTAL_WIDTH, HORIZONTAL_HEIGHT)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center),
        Orientation::Vertical => LayoutStyle::flex_column()
            .fixed_size(VERTICAL_WIDTH, VERTICAL_HEIGHT)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center),
    }
}

fn slider_thumb(interaction: InteractionState, disabled: Option<Signal<bool>>) -> NodeBuilder {
    let state_style = slider_thumb_state_style();
    let node = container()
        .layout(LayoutStyle::block().fixed_size(THUMB_SIZE, THUMB_SIZE))
        .class("rounded-md")
        .class(SLIDER_THUMB_ANIM_BASE)
        .style_reactive(move || {
            slider_thumb_variant().state(binary_style_state(interaction, disabled))
        });
    install_transition_animators(
        node,
        SLIDER_THUMB_ANIM_BASE,
        state_style,
        SLIDER_COMPONENT_ID,
        interaction,
    )
}

const fn slider_thumb_state_style() -> StateStyle {
    StateStyle::new(
        THUMB_REST,
        THUMB_HOVER,
        THUMB_HOVER,
        THUMB_DOWN,
        THUMB_DISABLED,
    )
}

fn noop_value_change() -> ValueChange {
    Rc::new(|_| {})
}

const fn slider_track_variant() -> StyleVariant {
    StyleVariant::new(
        TRACK,
        StateStyle::new(TRACK, TRACK, TRACK, TRACK, TRACK_DISABLED),
    )
}

const fn slider_thumb_variant() -> StyleVariant {
    StyleVariant::new(
        THUMB_REST,
        StateStyle::new(
            THUMB_REST,
            THUMB_HOVER,
            THUMB_HOVER,
            THUMB_DOWN,
            THUMB_DISABLED,
        ),
    )
}
