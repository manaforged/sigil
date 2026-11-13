use std::marker::PhantomData;
use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::events::PointerEvent;
use crate::layout::{AlignItems, LayoutStyle, Length, Orientation, cols, rows};
use crate::node::{
    AccessibleName, Action, Cx, IntoNode, NodeBuilder, NodeId, Role, TextContent, container,
};
use crate::reactive::{Reactive, Signal, signal};
use crate::style::{Metrics, Paint, Radius, States, StringKey, Theme, Token};
use crate::widgets::{Framed, Widget, WidgetFrame};

use super::handlers::{Grip, ThumbSignals, attach_key_handlers, attach_pointer_drag};
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
    frame: WidgetFrame,
}

impl<T> core::fmt::Debug for SliderBuilder<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SliderBuilder")
            .field("runtime", &self.runtime)
            .field("label", &self.label)
            .field("marks", &self.marks)
            .field("frame", &self.frame)
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
            frame: WidgetFrame::new(),
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
        let explicit_len = runtime.length.or_else(|| match self.frame.width() {
            Some(Length::Px(px)) => Some(px),
            _ => None,
        });
        let on_change: ValueChange = self.on_change.take().unwrap_or_else(|| Rc::new(|_| {}));
        let dragged = match runtime.value {
            Thumb::Start(range) | Thumb::End(range) => {
                super::range::range_root(runtime, range, self.label.as_deref(), &on_change)
            }
            Thumb::Only(_) => single_root(&self, runtime, Rc::clone(&on_change)),
        };
        let mut node = self.frame.finish(dragged);
        if let Some(caption) = self.label {
            node = super::caption::with_caption(node, caption, runtime.orientation, explicit_len);
        }
        if !self.marks.is_empty() && runtime.orientation != Orientation::Vertical {
            let marks_rt = SliderRuntime {
                length: explicit_len,
                ..runtime
            };
            node = super::marks::with_marks(node, marks_rt, self.marks, on_change);
        }
        node
    }
}

impl<T: 'static> Framed for SliderBuilder<T> {
    fn frame_mut(&mut self) -> &mut WidgetFrame {
        &mut self.frame
    }
}

impl<T: 'static> IntoNode for SliderBuilder<T> {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.root().build(cx)
    }
}

fn single_root<T>(
    builder: &SliderBuilder<T>,
    runtime: SliderRuntime,
    on_change: ValueChange,
) -> NodeBuilder {
    let signals = ThumbSignals {
        hovered: signal(false),
        focused: signal(false),
        pressed: signal(false),
        dragging: signal(false),
    };
    let extent = signal(crate::geometry::Size::ZERO);
    let root = control_frame(slider_node(builder), runtime.disabled)
        .focusable(true)
        .track_size(extent)
        .track_hovered(signals.hovered)
        .track_focused(signals.focused)
        .track_pressed(signals.pressed);
    let root = slider_parts(root, runtime, extent, signals);
    let keyed = attach_key_handlers(root, runtime, Rc::clone(&on_change));
    let grip = Grip::new(signals.dragging);
    attach_pointer_drag(keyed, runtime, [grip.clone(), grip], on_change)
}

pub(super) fn control_frame(node: NodeBuilder, disabled: Option<Signal<bool>>) -> NodeBuilder {
    let node = node
        .radius(Radius::Md)
        .hover(|s| s.background(Token::SurfaceHover))
        .pressed(|s| s.background(Token::SurfacePressed))
        .disabled_paint(|s| s.opacity(0.5));
    match disabled {
        Some(flag) => node.disabled(flag),
        None => node,
    }
}

impl SliderRange {
    pub(super) fn span(self) -> f32 {
        (self.max - self.min).max(f32::EPSILON)
    }
}

impl SliderRuntime {
    pub(super) fn disabled(self) -> bool {
        self.disabled.is_some_and(Signal::peek)
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
    let name = builder.label.as_ref().map_or_else(
        || AccessibleName::Derived(Theme::label(StringKey::Slider)),
        |label| AccessibleName::from(&TextContent::Static(label.clone())),
    );
    slider_semantics(runtime, name).layout(layout)
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
    extent: Signal<crate::geometry::Size>,
    signals: ThumbSignals,
) -> NodeBuilder {
    let thumb = slider_thumb(signals, runtime.disabled, runtime.glass);
    match runtime.orientation {
        Orientation::Horizontal => root
            .child(track(runtime, extent, true))
            .child(thumb)
            .child(track(runtime, extent, false)),
        Orientation::Vertical => root
            .child(track(runtime, extent, false))
            .child(thumb)
            .child(track(runtime, extent, true)),
    }
}

pub(super) fn slider_layout(
    orientation: Orientation,
    length: Option<f32>,
    captioned: bool,
) -> LayoutStyle {
    let grid = Metrics::current().is_grid();
    match orientation {
        Orientation::Horizontal => {
            let height = if grid {
                rows(1.0)
            } else {
                Length::from(HORIZONTAL_HEIGHT)
            };
            let base = LayoutStyle::flex_row().height(height);
            let base = match length {
                Some(px) => base.width(px),
                None if captioned => base.grow(1.0).width(0.0),
                None => base.fill_width(),
            };
            base.align_items(AlignItems::Center)
        }
        Orientation::Vertical => {
            let width = if grid {
                cols(1.0)
            } else {
                Length::from(VERTICAL_WIDTH)
            };
            let base = LayoutStyle::flex_column().width(width);
            let base = match length {
                Some(px) => base.height(px),
                None => base.fill_height(),
            };
            base.align_items(AlignItems::Center)
        }
    }
}
