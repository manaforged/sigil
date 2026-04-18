use std::ops::RangeInclusive;
use std::rc::Rc;

use sigil_core::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_events::prelude::{Key, KeyEvent, NodeEventsExt, PointerEvent};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::Signal;
use sigil_style::{ClassSource, NodeStyleExt, StateStyle, Style, StyleAtom, StyleVariant, Theme};

use crate::interaction::InteractionState;

use super::shared::{BuilderMutator, binary_style_state, disabled_now};

const HORIZONTAL_WIDTH: f32 = 180.0;
const HORIZONTAL_HEIGHT: f32 = 28.0;
const VERTICAL_WIDTH: f32 = 28.0;
const VERTICAL_HEIGHT: f32 = 180.0;

type ValueChange = Rc<dyn Fn(f32) + 'static>;

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
struct SliderRange {
    min: f32,
    max: f32,
}

#[derive(Clone, Copy)]
struct SliderRuntime {
    value: Signal<f32>,
    range: SliderRange,
    step: f32,
    disabled: Option<Signal<bool>>,
    orientation: Orientation,
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
}

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
    pub fn on_change(mut self, change: impl Fn(f32) + 'static) -> Self {
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

    /// Add struct-built style after the built-in slider defaults.
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
        node
    }

    fn base_node(&self, interaction: InteractionState, on_change: ValueChange) -> NodeBuilder {
        let runtime = SliderRuntime::from_builder(self);
        let node = slider_root(runtime, interaction)
            .on_pointer_down(move |event| slider_pointer_down(event, interaction, runtime))
            .on_pointer_move(move |event| slider_pointer_move(event, interaction, runtime))
            .on_pointer_up(move |event| {
                slider_pointer_up(event, interaction, runtime.value, &on_change);
            });
        interaction.install(slider_key_handlers(node, interaction, runtime))
    }
}

impl IntoNode for SliderBuilder {
    fn build(self, cx: &mut Cx) -> sigil_core::NodeId {
        self.into_node_builder().build(cx)
    }
}

impl SliderRange {
    const fn from_bounds(start: f32, end: f32) -> Self {
        Self {
            min: start.min(end),
            max: start.max(end),
        }
    }

    fn span(self) -> f32 {
        (self.max - self.min).max(f32::EPSILON)
    }

    const fn clamp(self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }
}

impl SliderRuntime {
    const fn from_builder(builder: &SliderBuilder) -> Self {
        Self {
            value: builder.value,
            range: builder.range,
            step: builder.step,
            disabled: builder.disabled,
            orientation: builder.orientation,
        }
    }

    fn disabled(self) -> bool {
        disabled_now(self.disabled)
    }

    fn set_from_position(self, event: &PointerEvent) {
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

    fn step_value(self, increase: bool) {
        let delta = normalized_step(self.step);
        let signed = if increase { delta } else { -delta };
        self.value
            .set(quantize(self.range, self.value.peek() + signed, self.step));
    }
}

fn slider_root(runtime: SliderRuntime, interaction: InteractionState) -> NodeBuilder {
    container()
        .role(Role::Slider)
        .aria_label("Slider")
        .aria_value_signal(runtime.value, runtime.range.min, runtime.range.max)
        .layout(slider_layout(runtime.orientation))
        .class("rounded-md")
        .class(TRACK)
        .style_reactive(move || {
            slider_track_variant().state(binary_style_state(interaction, runtime.disabled))
        })
        .child(slider_thumb(interaction, runtime.disabled))
}

fn slider_key_handlers(
    node: NodeBuilder,
    interaction: InteractionState,
    runtime: SliderRuntime,
) -> NodeBuilder {
    node.on_key(Key::ArrowRight, move |event| {
        slider_key(event, interaction, runtime, true);
    })
    .on_key(Key::ArrowUp, move |event| {
        slider_key(event, interaction, runtime, true);
    })
    .on_key(Key::ArrowLeft, move |event| {
        slider_key(event, interaction, runtime, false);
    })
    .on_key(Key::ArrowDown, move |event| {
        slider_key(event, interaction, runtime, false);
    })
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
    container()
        .layout(LayoutStyle::block().fixed_size(22.0, 22.0))
        .class("rounded-md")
        .style_reactive(move || {
            slider_thumb_variant().state(binary_style_state(interaction, disabled))
        })
}

fn slider_pointer_down(
    event: &mut PointerEvent,
    interaction: InteractionState,
    runtime: SliderRuntime,
) {
    if runtime.disabled() {
        return;
    }
    interaction.set_dragging(true);
    event.capture_pointer();
    runtime.set_from_position(event);
}

fn slider_pointer_move(
    event: &mut PointerEvent,
    interaction: InteractionState,
    runtime: SliderRuntime,
) {
    if runtime.disabled() || !event.is_captured() || !interaction.dragging() {
        return;
    }
    runtime.set_from_position(event);
    // Symmetry with slider_pointer_down: we've consumed the drag, no
    // reason to let an ancestor also react to a captured move.
    event.stop_propagation();
}

fn slider_pointer_up(
    event: &mut PointerEvent,
    interaction: InteractionState,
    value: Signal<f32>,
    on_change: &ValueChange,
) {
    let was_dragging = interaction.dragging();
    interaction.set_dragging(false);
    let _released = interaction.release_pointer(event);
    if was_dragging {
        on_change(value.peek());
    }
}

fn slider_key(
    event: &mut KeyEvent,
    interaction: InteractionState,
    runtime: SliderRuntime,
    increase: bool,
) {
    if !interaction.activate_key(event) || runtime.disabled() {
        return;
    }
    runtime.step_value(increase);
}

fn quantize(range: SliderRange, raw: f32, step: f32) -> f32 {
    let clamped = range.clamp(raw);
    let step = normalized_step(step);
    let steps = ((clamped - range.min) / step).round();
    range.clamp(steps.mul_add(step, range.min))
}

fn normalized_step(step: f32) -> f32 {
    if step.is_finite() && step > 0.0 {
        step
    } else {
        1.0
    }
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
