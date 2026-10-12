use std::ops::RangeInclusive;
use std::rc::Rc;

use accesskit::Action;

use crate::events::{NodeEventsExt, PointerEvent};
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, Dimension, LayoutBuilderExt, LayoutStyle, cols, rows};
use crate::reactive::{Reactive, Signal};
use crate::style::{NodeStyleExt, PaintTarget, StateHost, Style, StyleOverlay, StyleSlot, Theme};
use crate::widgets::interaction::InteractionState;
use crate::widgets::orientation::Orientation;

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::{BuilderMutator, control_surface, disabled_now};
use super::handlers::{attach_key_handlers, pointer_down, pointer_move, pointer_up};
use super::track::{slider_thumb, track};

pub(super) type ValueChange = Rc<dyn Fn(f32) + 'static>;

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
    runtime: SliderRuntime,
    on_change: Option<ValueChange>,
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
        let (start, end) = range.into_inner();
        Self {
            runtime: SliderRuntime {
                value,
                range: SliderRange {
                    min: start.min(end),
                    max: start.max(end),
                },
                step: 1.0,
                disabled: None,
                orientation: Orientation::Horizontal,
                length: None,
                glass: false,
            },
            on_change: None,
            label: None,
            marks: Vec::new(),
            mutators: Vec::new(),
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub const fn length(mut self, length: f32) -> Self {
        self.runtime.length = Some(length);
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
        self.runtime.step = step;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.runtime.disabled = Some(disabled.into_signal());
        self
    }

    #[must_use]
    pub fn change(mut self, change: impl Fn(f32) + 'static) -> Self {
        self.on_change = Some(Rc::new(change));
        self
    }

    #[must_use]
    pub const fn glass(mut self) -> Self {
        self.runtime.glass = true;
        self
    }

    #[must_use]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.runtime.orientation = orientation;
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        let runtime = self.runtime;
        let interaction = InteractionState::new(!runtime.disabled());
        let on_change: ValueChange = self.on_change.take().unwrap_or_else(|| Rc::new(|_| {}));
        let user_overlay = self.style_accumulator.take_state_overlay();
        let root = slider_root(&self, interaction, user_overlay);
        let change = Rc::clone(&on_change);
        let mut node = interaction.install(
            attach_key_handlers(root, interaction, runtime, Rc::clone(&on_change))
                .pointer_down(move |event| pointer_down(event, interaction, runtime))
                .pointer_move(move |event| pointer_move(event, interaction, runtime))
                .pointer_up(move |event| pointer_up(event, interaction, runtime.value, &change)),
        );
        for mutate in self.mutators {
            node = mutate(node);
        }
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        if let Some(caption) = self.label {
            node = super::caption::with_caption(node, caption, runtime.orientation, runtime.length);
        }
        if self.marks.is_empty() || runtime.orientation == Orientation::Vertical {
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

impl IntoNode for SliderBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

impl SliderRange {
    pub(super) fn span(self) -> f32 {
        (self.max - self.min).max(f32::EPSILON)
    }
}

impl SliderRuntime {
    pub(super) fn disabled(self) -> bool {
        disabled_now(self.disabled)
    }

    fn step_size(self) -> f32 {
        if self.step.is_finite() && self.step > 0.0 {
            self.step
        } else {
            1.0
        }
    }

    pub(super) fn quantize(self, raw: f32) -> f32 {
        let SliderRange { min, max } = self.range;
        let step = self.step_size();
        let steps = ((raw.clamp(min, max) - min) / step).round();
        steps.mul_add(step, min).clamp(min, max)
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
        self.value.set(self.quantize(raw));
    }

    pub(super) fn nudge(self, steps: f32) {
        let delta = self.step_size() * steps;
        self.value.set(self.quantize(self.value.peek() + delta));
    }
}

fn slider_root(
    builder: &SliderBuilder,
    interaction: InteractionState,
    overlay: StyleOverlay,
) -> NodeBuilder {
    let runtime = builder.runtime;
    let layout = slider_layout(runtime.orientation, runtime.length, builder.label.is_some());
    let root = container()
        .role(Role::Slider)
        .label(builder.label.clone().unwrap_or_else(|| "Slider".to_owned()))
        .action(Action::Increment)
        .action(Action::Decrement)
        .value(runtime.value, runtime.range.min, runtime.range.max)
        .value_step(runtime.step)
        .layout(builder.layout_overrides.clone().apply(layout));
    let root = control_surface(root, interaction, runtime.disabled, overlay);
    let thumb = slider_thumb(interaction, runtime.disabled, runtime.glass);
    match runtime.orientation {
        Orientation::Horizontal => root
            .child(track(runtime, true))
            .child(thumb)
            .child(track(runtime, false)),
        Orientation::Vertical => root
            .child(track(runtime, false))
            .child(thumb)
            .child(track(runtime, true)),
    }
}

fn slider_layout(orientation: Orientation, length: Option<f32>, captioned: bool) -> LayoutStyle {
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
                None if captioned => base.grow(1.0).width(0.0),
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
