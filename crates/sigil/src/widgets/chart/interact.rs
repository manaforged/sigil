use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

use crate::events::{
    MouseButton, MouseButtons, NodeEventsExt, PointerEvent, ScrollDelta, ScrollEvent,
};
use crate::kernel::{NodeBuilder, container, text};
use crate::layout::{LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{Signal, signal};
use crate::style::{Token, space};

use super::axis::format_tick;

const RULE: f32 = 1.0;
const READOUT_OFFSET: f32 = space(1);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Probe {
    x: f32,
    y: f32,
    width: f32,
}

#[derive(Clone)]
pub(super) struct Interact {
    range: Option<Signal<Range<f32>>>,
    crosshair: bool,
    probe: Signal<Option<Probe>>,
    width: Rc<Cell<f32>>,
    drag: Rc<Cell<Option<f32>>>,
}

impl Interact {
    pub(super) fn new(range: Option<Signal<Range<f32>>>, crosshair: bool) -> Self {
        Self {
            range,
            crosshair,
            probe: signal(None),
            width: Rc::new(Cell::new(0.0)),
            drag: Rc::new(Cell::new(None)),
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "the window is clamped to a non-negative point count"
    )]
    pub(super) fn window(&self, total: usize) -> Range<usize> {
        let Some(range) = self.range else {
            return 0..total;
        };
        let span = clamp_range(range.get(), total as f32);
        let start = (span.start.floor() as usize).min(total);
        let end = (span.end.ceil() as usize).min(total);
        if end <= start { 0..total } else { start..end }
    }

    pub(super) fn attach(
        &self,
        mut plot: NodeBuilder,
        total: usize,
        pad: f32,
        values: impl FnOnce() -> Rc<[f32]>,
    ) -> NodeBuilder {
        if self.crosshair {
            plot = plot.child(self.overlay(values(), pad));
        }
        if self.crosshair || self.range.is_some() {
            plot = self.pointer(plot, total, pad);
            plot = self.wheel(plot, total, pad);
        }
        plot
    }

    fn pointer(&self, plot: NodeBuilder, total: usize, pad: f32) -> NodeBuilder {
        let (probe, crosshair, range) = (self.probe, self.crosshair, self.range);
        let width = Rc::clone(&self.width);
        let (drag, down_drag, up_drag) = (
            Rc::clone(&self.drag),
            Rc::clone(&self.drag),
            Rc::clone(&self.drag),
        );
        plot.pointer_move(move |event: &mut PointerEvent| {
            width.set(event.target_size.width);
            if crosshair {
                probe.set(Some(Probe {
                    x: event.position.x,
                    y: event.position.y,
                    width: event.target_size.width,
                }));
            }
            let (Some(range), Some(last)) = (range, drag.get()) else {
                return;
            };
            if !event.buttons.contains(MouseButtons::LEFT) {
                drag.set(None);
                return;
            }
            let moved = (event.position.x - last) / span_px(event.target_size.width, pad);
            pan(range, total, moved);
            drag.set(Some(event.position.x));
        })
        .pointer_down(move |event: &mut PointerEvent| {
            if range.is_some() && event.button == Some(MouseButton::Left) {
                down_drag.set(Some(event.position.x));
                event.capture_pointer();
            }
        })
        .pointer_up(move |_: &mut PointerEvent| up_drag.set(None))
        .pointer_leave(move |_: &mut PointerEvent| probe.set(None))
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "point counts stay far below f32 precision limits at render scale"
    )]
    fn wheel(&self, plot: NodeBuilder, total: usize, pad: f32) -> NodeBuilder {
        let Some(range) = self.range else {
            return plot;
        };
        let width = Rc::clone(&self.width);
        plot.on_scroll(move |event: &mut ScrollEvent| {
            let steps = match event.delta {
                ScrollDelta::Pixels(delta) => delta.y * 0.002,
                ScrollDelta::Lines(delta) => delta.y * 0.1,
            };
            if steps == 0.0 || !steps.is_finite() {
                return;
            }
            let at = ((event.position.x - pad) / span_px(width.get(), pad)).clamp(0.0, 1.0);
            range.update(|current| {
                let full = total as f32;
                let span = clamp_range(current.clone(), full);
                let len = (span.end - span.start) * steps.exp();
                let start = len.mul_add(-at, (span.end - span.start).mul_add(at, span.start));
                *current = clamp_range(start..start + len, full);
            });
            event.stop_propagation();
        })
    }

    fn overlay(&self, values: Rc<[f32]>, pad: f32) -> NodeBuilder {
        let probe = self.probe;
        container()
            .layout(
                LayoutStyle::block()
                    .position(Position::Absolute)
                    .inset_top(0.0)
                    .inset_left(0.0)
                    .fill(),
            )
            .children_reactive(move || {
                probe
                    .get()
                    .map_or_else(Vec::new, |at| marks(at, &values, pad))
            })
    }
}

fn span_px(width: f32, pad: f32) -> f32 {
    2.0f32.mul_add(-pad, width).max(1.0)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "point counts stay far below f32 precision limits at render scale"
)]
fn pan(range: Signal<Range<f32>>, total: usize, moved: f32) {
    range.update(|current| {
        let full = total as f32;
        let span = clamp_range(current.clone(), full);
        let shift = -moved * (span.end - span.start);
        *current = clamp_range(span.start + shift..span.end + shift, full);
    });
}

fn clamp_range(range: Range<f32>, total: f32) -> Range<f32> {
    if !(range.start.is_finite() && range.end.is_finite()) || range.end <= range.start {
        return 0.0..total;
    }
    let len = (range.end - range.start).clamp(2.0f32.min(total), total);
    let start = range.start.clamp(0.0, (total - len).max(0.0));
    start..start + len
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the probe index is clamped to the visible point count"
)]
fn marks(at: Probe, values: &[f32], pad: f32) -> Vec<NodeBuilder> {
    let count = values.len();
    let fraction = ((at.x - pad) / span_px(at.width, pad)).clamp(0.0, 1.0);
    let index = fraction
        .mul_add(count as f32, -0.5)
        .round()
        .clamp(0.0, count.saturating_sub(1) as f32) as usize;
    let mut nodes = vec![
        rule(
            LayoutStyle::block()
                .inset_left(at.x)
                .inset_top(0.0)
                .width(RULE)
                .fill_height(),
        ),
        rule(
            LayoutStyle::block()
                .inset_left(0.0)
                .inset_top(at.y)
                .height(RULE)
                .fill_width(),
        ),
    ];
    if let Some(value) = values.get(index) {
        nodes.push(
            container()
                .layout(
                    LayoutStyle::block()
                        .position(Position::Absolute)
                        .inset_left(at.x + READOUT_OFFSET)
                        .inset_top(at.y + READOUT_OFFSET),
                )
                .class([Token::Surface.bg()])
                .child(
                    text(format_tick(*value))
                        .nowrap()
                        .class("text-xs")
                        .class(Token::Gray12.text()),
                ),
        );
    }
    nodes
}

fn rule(layout: LayoutStyle) -> NodeBuilder {
    container()
        .layout(layout.position(Position::Absolute))
        .class([Token::Gray8.bg()])
}
