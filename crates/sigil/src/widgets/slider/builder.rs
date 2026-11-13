use std::marker::PhantomData;
use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::kernel::Action;

use crate::events::PointerEvent;
use crate::kernel::{AccessibleName, NodeBuilder, Role, TextContent, container};
use crate::layout::Orientation;
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, LayoutStyle, Length, cols, rows};
use crate::reactive::{Reactive, Signal};
use crate::style::Style;
use crate::style::strings::StringKey;
use crate::widgets::Layout;
use crate::widgets::interaction::InteractionState;

use super::super::layout_ext::LayoutOverrides;
use super::super::shared::{accessible_name_or, caller_style, control, disabled_now};
use super::handlers::{Grip, attach_key_handlers, attach_pointer_drag};
use super::thumb::Thumb;
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

#[derive(Clone, Copy, Debug)]
pub(super) struct SliderRuntime {
    pub(super) value: Thumb,
    pub(super) range: SliderRange,
    pub(super) step: f32,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) orientation: Orientation,
    pub(super) length: Option<f32>,
    pub(super) glass: bool,
}

/// Builder of a slider; `T` is the value it edits, `f32` or a range.
#[must_use]
pub struct SliderBuilder<T = f32> {
    value: PhantomData<fn() -> T>,
    runtime: SliderRuntime,
    on_change: Option<ValueChange>,
    label: Option<String>,
    marks: Vec<String>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

impl<T> core::fmt::Debug for SliderBuilder<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SliderBuilder")
            .field("runtime", &self.runtime)
            .field("label", &self.label)
            .field("marks", &self.marks)
            .field("style_accumulator", &self.style_accumulator)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A value slider.
pub fn slider(value: Signal<f32>, range: RangeInclusive<f32>) -> SliderBuilder {
    SliderBuilder::new(value, range)
}

/// A slider with two thumbs that edits a range inside `range`; the start thumb never passes the end thumb.
///
/// ```
/// use sigil::prelude::*;
/// use sigil::widgets::range_slider;
///
/// fn price_filter() -> impl IntoNode {
///     let price = signal(20.0_f32..=80.0);
///     range_slider(price, 0.0..=100.0).step(5.0).label("Price")
/// }
/// ```
pub fn range_slider(
    value: Signal<RangeInclusive<f32>>,
    range: RangeInclusive<f32>,
) -> SliderBuilder<RangeInclusive<f32>> {
    SliderBuilder::with_thumb(Thumb::Start(value), range)
}

impl SliderBuilder {
    /// Creates a slider that edits `value` inside `range` with a step of 1.
    pub(crate) fn new(value: Signal<f32>, range: RangeInclusive<f32>) -> Self {
        Self::with_thumb(Thumb::Only(value), range)
    }

    /// Calls `change` with the value after each edit.
    pub fn change(self, change: impl Fn(f32) + 'static) -> Self {
        self.on_change(Rc::new(change))
    }
}

impl SliderBuilder<RangeInclusive<f32>> {
    /// Calls `change` with the whole range after either thumb moves.
    pub fn change(self, change: impl Fn(RangeInclusive<f32>) + 'static) -> Self {
        let (Thumb::Start(range) | Thumb::End(range)) = self.runtime.value else {
            return self;
        };
        self.on_change(Rc::new(move |_| change(range.peek())))
    }
}

impl<T> SliderBuilder<T> {
    fn with_thumb(value: Thumb, range: RangeInclusive<f32>) -> Self {
        let (start, end) = range.into_inner();
        Self {
            value: PhantomData,
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
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Sets the length along the axis in px; without it, the slider fills its parent.
    pub const fn length(mut self, length: f32) -> Self {
        self.runtime.length = Some(length);
        self
    }

    /// Sets the accessible name and shows it as a caption.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Shows `marks` as labels spread along a horizontal slider.
    pub fn marks(mut self, marks: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.marks = marks.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the increment that the value snaps to; the default is 1.
    pub const fn step(mut self, step: f32) -> Self {
        self.runtime.step = step;
        self
    }

    /// Blocks edits while `disabled` is true.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.runtime.disabled = Some(disabled.into_signal());
        self
    }

    fn on_change(mut self, change: ValueChange) -> Self {
        self.on_change = Some(change);
        self
    }

    /// Draws the track and the thumb with gradient fills.
    pub const fn glass(mut self) -> Self {
        self.runtime.glass = true;
        self
    }

    /// Sets the axis of the slider; the default is `Orientation::Horizontal`.
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.runtime.orientation = orientation;
        self
    }

    fn root(mut self) -> NodeBuilder {
        let runtime = self.runtime;
        let on_change: ValueChange = self.on_change.take().unwrap_or_else(|| Rc::new(|_| {}));
        let style = std::mem::take(&mut self.style_accumulator);
        let dragged = match runtime.value {
            Thumb::Start(range) | Thumb::End(range) => {
                super::range::range_root(runtime, range, self.label.as_deref(), &on_change)
            }
            Thumb::Only(_) => {
                let mut root = control(slider_node(&self), runtime.disabled).active();
                let interaction = root.interaction();
                let root = slider_parts(root, runtime, interaction);
                let keyed = attach_key_handlers(root, interaction, runtime, Rc::clone(&on_change));
                let grip = Grip::new(interaction);
                attach_pointer_drag(keyed, runtime, [grip.clone(), grip], Rc::clone(&on_change))
            }
        };
        let mut node = caller_style(dragged, style);
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

builder_impls!([T: 'static] SliderBuilder<T>: paint(style_accumulator), widget, states(style_accumulator), node);

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

    pub(super) fn value_at(self, event: &PointerEvent) -> f32 {
        let (extent, fallback) = match self.orientation {
            Orientation::Horizontal => (event.target_size.width, HORIZONTAL_WIDTH),
            Orientation::Vertical => (event.target_size.height, VERTICAL_HEIGHT),
        };
        let extent = if extent > 0.0 { extent } else { fallback };
        let ratio = match self.orientation {
            Orientation::Horizontal => event.position.x / extent,
            Orientation::Vertical => 1.0 - (event.position.y / extent),
        };
        self.quantize(
            self.range
                .span()
                .mul_add(ratio.clamp(0.0, 1.0), self.range.min),
        )
    }

    pub(super) fn ratio(self) -> f32 {
        (self.value.get() - self.range.min) / self.range.span()
    }

    pub(super) const fn on(self, value: Thumb) -> Self {
        Self { value, ..self }
    }

    pub(super) fn nudge(self, steps: f32) {
        let delta = self.step_size() * steps;
        self.value.set(self.quantize(self.value.peek() + delta));
    }
}

fn slider_node<T>(builder: &SliderBuilder<T>) -> NodeBuilder {
    let runtime = builder.runtime;
    let layout = slider_layout(runtime.orientation, runtime.length, builder.label.is_some());
    let label = builder.label.clone().map(TextContent::Static);
    slider_semantics(
        runtime,
        accessible_name_or(label.as_ref(), StringKey::Slider),
    )
    .layout(layout)
}

pub(super) fn slider_semantics(runtime: SliderRuntime, name: AccessibleName) -> NodeBuilder {
    container()
        .role(Role::Slider)
        .label(name)
        .action(Action::Increment)
        .action(Action::Decrement)
        .value(runtime.value.signal(), runtime.range.min, runtime.range.max)
        .value_step(runtime.step)
}

fn slider_parts(
    root: NodeBuilder,
    runtime: SliderRuntime,
    interaction: InteractionState,
) -> NodeBuilder {
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

pub(super) fn slider_layout(
    orientation: Orientation,
    length: Option<f32>,
    captioned: bool,
) -> LayoutStyle {
    match orientation {
        Orientation::Horizontal => {
            let base = LayoutStyle::flex_row()
                .height(CellForm::new(Length::from(HORIZONTAL_HEIGHT), rows(1.0)).pick());
            let base = match length {
                Some(px) => base.width(px),
                None if captioned => base.grow(1.0).width(0.0),
                None => base.fill_width(),
            };
            base.align_items(AlignItems::Center)
        }
        Orientation::Vertical => {
            let base = LayoutStyle::flex_column()
                .width(CellForm::new(Length::from(VERTICAL_WIDTH), cols(1.0)).pick());
            let base = match length {
                Some(px) => base.height(px),
                None => base.fill_height(),
            };
            base.align_items(AlignItems::Center)
        }
    }
}
