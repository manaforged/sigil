use crate::reactive::Reactive;
use std::ops::RangeInclusive;
use std::rc::Rc;

use accesskit::Action;

use crate::events::{NodeEventsExt, PointerEvent};
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, Dimension, LayoutBuilderExt, LayoutStyle, cols, rows};
use crate::reactive::Signal;
use crate::style::{NodeStyleExt, PaintTarget, StateHost, Style, StyleOverlay, StyleSlot, Theme};

use crate::widgets::interaction::InteractionState;

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::{BuilderMutator, control_surface, disabled_now};
use super::handlers::{
    attach_key_handlers, normalized_step, pointer_down, pointer_move, pointer_up, quantize,
};
use super::track::{slider_thumb, track_fill, track_rest};

pub(super) type ValueChange = Rc<dyn Fn(f32) + 'static>;

fn noop_value_change() -> ValueChange {
    Rc::new(|_| {})
}

use crate::widgets::orientation::Orientation;

const HORIZONTAL_WIDTH: f32 = 180.0;
const HORIZONTAL_HEIGHT: f32 = 24.0;
const VERTICAL_WIDTH: f32 = 24.0;
const VERTICAL_HEIGHT: f32 = 180.0;

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
    pub(super) length: Option<f32>,
    pub(super) glass: bool,
}

pub struct SliderBuilder {
    value: Signal<f32>,
    range: SliderRange,
    step: f32,
    disabled: Option<Signal<bool>>,
    on_change: Option<ValueChange>,
    orientation: Orientation,
    length: Option<f32>,
    glass: bool,
    label: Option<String>,
    marks: Vec<String>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn slider(value: Signal<f32>, range: RangeInclusive<f32>) -> SliderBuilder {
    SliderBuilder::new(value, range)
}

impl SliderBuilder {
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
            length: None,
            glass: false,
            label: None,
            marks: Vec::new(),
            mutators: Vec::new(),
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub const fn length(mut self, length: f32) -> Self {
        self.length = Some(length);
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn marks(mut self, marks: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.marks = marks.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub const fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    #[must_use]
    pub fn change(mut self, change: impl Fn(f32) + 'static) -> Self {
        self.on_change = Some(Rc::new(change));
        self
    }

    #[must_use]
    pub const fn glass(mut self) -> Self {
        self.glass = true;
        self
    }

    #[must_use]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

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
            .map_or_else(noop_value_change, Rc::clone);
        let runtime = SliderRuntime::from_builder(&self);
        let user_overlay = self.style_accumulator.take_state_overlay();
        let mut node = self.base_node(interaction, Rc::clone(&on_change), user_overlay);
        for mutate in self.mutators {
            node = mutate(node);
        }
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        if self.marks.is_empty() || self.orientation == Orientation::Vertical {
            node
        } else {
            super::marks::with_marks(node, runtime, self.marks, on_change)
        }
    }
}

impl PaintTarget for SliderBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl StateHost for SliderBuilder {}

impl WidgetLayoutExt for SliderBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl SliderBuilder {
    fn base_node(
        &self,
        interaction: InteractionState,
        on_change: ValueChange,
        user_overlay: StyleOverlay,
    ) -> NodeBuilder {
        let runtime = SliderRuntime::from_builder(self);
        let node = attach_key_handlers(
            slider_root(
                runtime,
                interaction,
                user_overlay,
                self.accessible_label(),
                self.layout_overrides.clone(),
            ),
            interaction,
            runtime,
            Rc::clone(&on_change),
        )
        .pointer_down(move |event| pointer_down(event, interaction, runtime))
        .pointer_move(move |event| pointer_move(event, interaction, runtime))
        .pointer_up(move |event| {
            pointer_up(event, interaction, runtime.value, &on_change);
        });
        interaction.install(node)
    }
}

impl SliderBuilder {
    fn accessible_label(&self) -> String {
        self.label.clone().unwrap_or_else(|| "Slider".to_owned())
    }
}

impl IntoNode for SliderBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
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
            length: builder.length,
            glass: builder.glass,
        }
    }

    pub(super) fn disabled(self) -> bool {
        disabled_now(self.disabled)
    }

    pub(super) fn set_from_position(self, event: &PointerEvent) {
        let (extent, fallback) = match self.orientation {
            Orientation::Horizontal => (event.target_size.width, HORIZONTAL_WIDTH),
            Orientation::Vertical => (event.target_size.height, VERTICAL_HEIGHT),
        };
        let extent = if extent > 0.0 { extent } else { fallback };
        let ratio = match self.orientation {
            Orientation::Horizontal => event.position.x / extent,
            Orientation::Vertical => 1.0 - (event.position.y / extent),
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

fn slider_root(
    runtime: SliderRuntime,
    interaction: InteractionState,
    user_overlay: StyleOverlay,
    label: String,
    overrides: LayoutOverrides,
) -> NodeBuilder {
    let root = container()
        .role(Role::Slider)
        .label(label)
        .action(Action::Increment)
        .action(Action::Decrement)
        .value(runtime.value, runtime.range.min, runtime.range.max)
        .value_step(runtime.step)
        .layout(overrides.apply(slider_layout(runtime.orientation, runtime.length)));
    let root = control_surface(root, interaction, runtime.disabled, user_overlay);
    match runtime.orientation {
        Orientation::Horizontal => root
            .child(track_fill(runtime))
            .child(slider_thumb(interaction, runtime.disabled, runtime.glass))
            .child(track_rest(runtime)),
        Orientation::Vertical => root
            .child(track_rest(runtime))
            .child(slider_thumb(interaction, runtime.disabled, runtime.glass))
            .child(track_fill(runtime)),
    }
}

fn slider_layout(orientation: Orientation, length: Option<f32>) -> LayoutStyle {
    match orientation {
        Orientation::Horizontal => {
            let base = LayoutStyle::flex_row().height(
                CellForm::new(
                    Dimension::from(HORIZONTAL_HEIGHT),
                    Dimension::from(rows(1.0)),
                )
                .pick(),
            );
            let base = match length {
                Some(px) => base.width(px),
                None => base.fill_width(),
            };
            base.align_items(AlignItems::Center)
        }
        Orientation::Vertical => {
            let base = LayoutStyle::flex_column().width(
                CellForm::new(Dimension::from(VERTICAL_WIDTH), Dimension::from(cols(1.0))).pick(),
            );
            let base = match length {
                Some(px) => base.height(px),
                None => base.fill_height(),
            };
            base.align_items(AlignItems::Center)
        }
    }
}
