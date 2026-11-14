use std::cell::Cell;
use std::fmt;
use std::ops::RangeInclusive;
use std::rc::Rc;

use sigil::animation::{Property, Transition};
use sigil::events::{A11yEdit, DragPhase, KeyEvent, NamedKey, PointerEvent, ValueStep};
use sigil::layout::{LayoutStyle, cols};
use sigil::node::{Action, Cx, NodeId};
use sigil::prelude::*;
use sigil::style::{BORDER_WIDTH, Elevation, Metrics, StringKey};
use sigil::widgets::{Framed, WidgetFrame};

const SLIDER_KEYS: [(NamedKey, ValueStep); 8] = [
    (NamedKey::ArrowRight, ValueStep::Unit(1.0)),
    (NamedKey::ArrowUp, ValueStep::Unit(1.0)),
    (NamedKey::ArrowLeft, ValueStep::Unit(-1.0)),
    (NamedKey::ArrowDown, ValueStep::Unit(-1.0)),
    (NamedKey::PageUp, ValueStep::Page(1.0)),
    (NamedKey::PageDown, ValueStep::Page(-1.0)),
    (NamedKey::Home, ValueStep::Start),
    (NamedKey::End, ValueStep::End),
];

const PAGE_STEPS: f32 = 10.0;

#[derive(Clone, Copy)]
struct SliderRuntime {
    value: Signal<f32>,
    min: f32,
    max: f32,
    step: f32,
    disabled: Option<Signal<bool>>,
}

impl SliderRuntime {
    fn is_disabled(self) -> bool {
        self.disabled.is_some_and(Signal::peek)
    }
    fn span(self) -> f32 {
        (self.max - self.min).max(f32::EPSILON)
    }
    fn ratio(self) -> f32 {
        ((self.value.get() - self.min) / self.span()).clamp(0.0, 1.0)
    }
    fn quantize(self, raw: f32) -> f32 {
        ((raw.clamp(self.min, self.max) - self.min) / self.step)
            .round()
            .mul_add(self.step, self.min)
            .clamp(self.min, self.max)
    }
    fn commit(self, next: f32, cb: &Rc<dyn Fn(f32)>) {
        self.value.set(next);
        cb(self.value.peek());
    }
    fn value_at(self, event: &PointerEvent) -> f32 {
        let w = event.target_size().width;
        let extent = if w > 0.0 { w } else { 180.0 };
        self.quantize(
            self.span()
                .mul_add((event.position().x / extent).clamp(0.0, 1.0), self.min),
        )
    }
}

/// External reimplementation of `slider` using only public Layer 1 and Layer 2 APIs.
#[must_use]
pub struct CustomSliderBuilder {
    runtime: SliderRuntime,
    length: Option<f32>,
    label: Option<String>,
    on_change: Option<Rc<dyn Fn(f32)>>,
    frame: WidgetFrame,
}

/// Creates a custom horizontal slider bound to `value` within `range`.
pub fn custom_slider(value: Signal<f32>, range: RangeInclusive<f32>) -> CustomSliderBuilder {
    let (start, end) = range.into_inner();
    CustomSliderBuilder {
        runtime: SliderRuntime {
            value,
            min: start.min(end),
            max: start.max(end),
            step: 1.0,
            disabled: None,
        },
        length: None,
        label: None,
        on_change: None,
        frame: WidgetFrame::new(),
    }
}

impl fmt::Debug for CustomSliderBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomSliderBuilder")
            .finish_non_exhaustive()
    }
}

impl Framed for CustomSliderBuilder {
    fn frame_mut(&mut self) -> &mut WidgetFrame {
        &mut self.frame
    }
}

impl IntoNode for CustomSliderBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.root().build(cx)
    }
}

impl CustomSliderBuilder {
    /// Sets the step increment; defaults to `1.0`.
    pub const fn step(mut self, step: f32) -> Self {
        self.runtime.step = if step.is_finite() && step > 0.0 {
            step
        } else {
            1.0
        };
        self
    }

    /// Disables the slider while `disabled` is true.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.runtime.disabled = Some(disabled.into_signal());
        self
    }

    /// Sets a fixed width in logical pixels; fills available width otherwise.
    pub const fn length(mut self, length: f32) -> Self {
        self.length = Some(length);
        self
    }

    /// Sets the accessible label on the slider.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Calls `change` whenever an edit commits a new value.
    pub fn change(mut self, change: impl Fn(f32) + 'static) -> Self {
        self.on_change = Some(Rc::new(change));
        self
    }

    fn root(mut self) -> NodeBuilder {
        let rt = self.runtime;
        let on_change = self.on_change.take().unwrap_or_else(|| Rc::new(|_| {}));
        let (hovered, focused, pressed, dragging) =
            (signal(false), signal(false), signal(false), signal(false));
        let name = self.label.clone().map_or_else(
            || AccessibleName::from(Theme::label(StringKey::Slider)),
            AccessibleName::from,
        );
        let length = self.length.or_else(|| match self.frame.width() {
            Some(Length::Px(px)) => Some(px),
            _ => None,
        });
        let row = slider_row_layout(self.length, self.label.is_some());
        let extent = signal(Size::new(0.0, 0.0));
        let (filled, rest) = slider_track_parts(rt, extent, Metrics::current().track_size(6.0));
        let mut node = container()
            .role(Role::Slider)
            .label(name)
            .action(Action::Increment)
            .action(Action::Decrement)
            .value(rt.value, rt.min, rt.max)
            .value_step(rt.step)
            .layout(row)
            .track_size(extent)
            .radius(Radius::Md)
            .hover(|s| s.background(Token::SurfaceHover))
            .pressed(|s| s.background(Token::SurfacePressed))
            .disabled_paint(|s| s.opacity(0.5))
            .focusable(true)
            .track_hovered(hovered)
            .track_focused(focused)
            .track_pressed(pressed)
            .child(filled)
            .child(slider_thumb(
                hovered,
                focused,
                pressed,
                dragging,
                rt.disabled,
            ))
            .child(rest);
        if let Some(disabled) = rt.disabled {
            node = node.disabled(disabled);
        }
        let slider = self
            .frame
            .finish(attach_slider_events(node, rt, dragging, on_change));
        with_caption(slider, self.label, length)
    }
}

fn with_caption(slider: NodeBuilder, caption: Option<String>, length: Option<f32>) -> NodeBuilder {
    let Some(caption) = caption else {
        return slider;
    };
    let base = LayoutStyle::flex_row();
    let base = if length.is_some() {
        base
    } else {
        base.fill_width()
    };
    container()
        .layout(
            base.gap(Space::raw(
                Metrics::current().control_gap(ControlSize::Default),
                "control gap",
            ))
            .align_items(AlignItems::Center),
        )
        .child(text(caption).nowrap())
        .child(slider)
}

fn slider_row_layout(length: Option<f32>, captioned: bool) -> LayoutStyle {
    let h = if Metrics::current().is_grid() {
        rows(1.0)
    } else {
        Length::from(24.0)
    };
    let row = LayoutStyle::flex_row().height(h);
    match length {
        Some(px) => row.width(px),
        None if captioned => row.grow(1.0).width(0.0),
        None => row.fill_width(),
    }
    .align_items(AlignItems::Center)
}

fn slider_track_parts(
    rt: SliderRuntime,
    extent: Signal<Size>,
    thickness: f32,
) -> (NodeBuilder, NodeBuilder) {
    let filled_px = memo(move || rt.ratio() * extent.get().width);
    let filled = container()
        .hittable(false)
        .radius(Radius::Full)
        .background(Token::Accent)
        .layout(LayoutStyle::block().height(thickness))
        .drive(Property::Width, filled_px);
    let rest = container()
        .hittable(false)
        .radius(Radius::Full)
        .background(Tone::Neutral.step(4))
        .layout(LayoutStyle::block().grow(1.0).height(thickness));
    (filled, rest)
}

fn slider_thumb(
    hovered: Signal<bool>,
    focused: Signal<bool>,
    pressed: Signal<bool>,
    dragging: Signal<bool>,
    disabled: Option<Signal<bool>>,
) -> NodeBuilder {
    if Metrics::current().is_grid() {
        return container()
            .hittable(false)
            .layout(
                LayoutStyle::block()
                    .width(cols(1.0))
                    .height(rows(1.0))
                    .shrink(0.0),
            )
            .text_color(Token::Accent)
            .child(text("●").hittable(false));
    }
    let active = memo(move || {
        (hovered.get() || focused.get() || pressed.get() || dragging.get())
            && !disabled.is_some_and(Signal::get)
    });
    container()
        .hittable(false)
        .layout(LayoutStyle::block().fixed_size(16.0, 16.0))
        .radius(Radius::Lg)
        .transition(Transition::smooth())
        .border_width(BORDER_WIDTH)
        .background(Token::OnAccent)
        .border_color(Token::Accent)
        .paint_when(active, |s| s.border_color(Token::AccentHover))
        .shadow(Elevation::Sm)
}

fn attach_slider_drag(
    node: NodeBuilder,
    rt: SliderRuntime,
    dragging: Signal<bool>,
    drag_change: Rc<dyn Fn(f32)>,
) -> NodeBuilder {
    let start_value = Rc::new(Cell::new(None::<f32>));
    node.pointer_drag(move |phase, event| match phase {
        DragPhase::Start if rt.is_disabled() => event.prevent_default(),
        DragPhase::Start => {
            start_value.set(Some(rt.value.peek()));
            dragging.set(true);
            rt.value.set(rt.value_at(event));
        }
        DragPhase::Move if start_value.get().is_some() && !rt.is_disabled() => {
            rt.value.set(rt.value_at(event));
            event.stop_propagation();
        }
        DragPhase::End if start_value.take().is_some() => {
            dragging.set(false);
            drag_change(rt.value.peek());
        }
        DragPhase::Cancel => {
            if let Some(initial) = start_value.take() {
                dragging.set(false);
                rt.value.set(initial);
            }
        }
        _ => {}
    })
}

fn attach_slider_events(
    node: NodeBuilder,
    rt: SliderRuntime,
    dragging: Signal<bool>,
    on_change: Rc<dyn Fn(f32)>,
) -> NodeBuilder {
    let (key_change, drag_change) = (Rc::clone(&on_change), Rc::clone(&on_change));
    attach_slider_drag(node, rt, dragging, drag_change)
        .step_keys(SLIDER_KEYS, move |step, event: &mut KeyEvent| {
            if rt.is_disabled() {
                return;
            }
            let next = match step {
                ValueStep::Unit(steps) => rt.quantize(rt.step.mul_add(steps, rt.value.peek())),
                ValueStep::Page(pages) => {
                    rt.quantize((rt.step * PAGE_STEPS).mul_add(pages, rt.value.peek()))
                }
                ValueStep::Start => rt.min,
                ValueStep::End => rt.max,
                _ => return,
            };
            event.stop_propagation();
            rt.commit(next, &key_change);
        })
        .a11y_edit(move |edit| match edit {
            A11yEdit::Increment | A11yEdit::Decrement => {
                if !rt.is_disabled() {
                    let sign = if matches!(edit, A11yEdit::Increment) {
                        1.0
                    } else {
                        -1.0
                    };
                    rt.commit(
                        rt.quantize(rt.step.mul_add(sign, rt.value.peek())),
                        &on_change,
                    );
                }
                true
            }
            A11yEdit::Number(raw) => {
                if !rt.is_disabled() {
                    rt.commit(rt.quantize(*raw), &on_change);
                }
                true
            }
            _ => false,
        })
}
