use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

use crate::events::{PointerEvent, ScrollDelta, ScrollEvent};
use crate::kernel::{NodeBuilder, container};
use crate::layout::{LayoutStyle, Position};
use crate::reactive::{Signal, signal};
use crate::style::Token;
use crate::widgets::Widget;
use crate::widgets::interaction::is_primary;

use super::tips::{Probe, Tips, card};

const ZOOM_PER_PIXEL: f32 = 0.002;
const ZOOM_PER_LINE: f32 = 0.1;
const MIN_SPAN: f32 = 2.0;
const RULE: f32 = 1.0;

#[derive(Clone)]
pub(super) struct Interact {
    range: Option<Signal<Range<f32>>>,
    crosshair: bool,
    tooltip: bool,
    probe: Signal<Option<Probe>>,
    width: Rc<Cell<f32>>,
    drag: Rc<Cell<Option<f32>>>,
}

impl Interact {
    pub(super) fn new(range: Option<Signal<Range<f32>>>, crosshair: bool, tooltip: bool) -> Self {
        Self {
            range,
            crosshair,
            tooltip,
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
        tips: impl FnOnce() -> Tips,
    ) -> NodeBuilder {
        let hover = self.crosshair || self.tooltip;
        if hover {
            plot = plot.child(self.overlay(tips(), pad));
        }
        if hover || self.range.is_some() {
            plot = self.pointer(plot, total, pad);
            plot = self.wheel(plot, total, pad);
        }
        plot
    }

    fn pointer(&self, plot: NodeBuilder, total: usize, pad: f32) -> NodeBuilder {
        let (probe, hover, range) = (self.probe, self.crosshair || self.tooltip, self.range);
        let width = Rc::clone(&self.width);
        let (drag, down_drag, up_drag) = (
            Rc::clone(&self.drag),
            Rc::clone(&self.drag),
            Rc::clone(&self.drag),
        );
        plot.pointer_move(move |event: &mut PointerEvent| {
            width.set(event.target_size.width);
            if hover {
                probe.set(Some(Probe {
                    x: event.position.x,
                    y: event.position.y,
                    width: event.target_size.width,
                    height: event.target_size.height,
                }));
            }
            let (Some(range), Some(last)) = (range, drag.get()) else {
                return;
            };
            if !event.is_captured() {
                drag.set(None);
                return;
            }
            let moved = (event.position.x - last) / span_px(event.target_size.width, pad);
            pan(range, total, moved);
            drag.set(Some(event.position.x));
        })
        .pointer_down(move |event: &mut PointerEvent| {
            if range.is_some() && is_primary(event) {
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
        plot.scroll(move |event: &mut ScrollEvent| {
            let steps = match event.delta {
                ScrollDelta::Pixels(delta) => delta.y * ZOOM_PER_PIXEL,
                ScrollDelta::Lines(delta) => delta.y * ZOOM_PER_LINE,
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

    fn overlay(&self, tips: Tips, pad: f32) -> NodeBuilder {
        let (probe, crosshair) = (self.probe, self.crosshair);
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
                    .map_or_else(Vec::new, |at| marks(at, &tips, pad, crosshair))
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
    let len = (range.end - range.start).clamp(MIN_SPAN.min(total), total);
    let start = range.start.clamp(0.0, (total - len).max(0.0));
    start..start + len
}

fn marks(at: Probe, tips: &Tips, pad: f32, crosshair: bool) -> Vec<NodeBuilder> {
    let mut nodes = Vec::new();
    if crosshair {
        nodes.push(rule(
            LayoutStyle::block()
                .inset_left(at.x)
                .inset_top(0.0)
                .width(RULE)
                .fill_height(),
        ));
        nodes.push(rule(
            LayoutStyle::block()
                .inset_left(0.0)
                .inset_top(at.y)
                .height(RULE)
                .fill_width(),
        ));
    }
    let lines = tips.index(at, pad).map(|index| (tips.lines)(index));
    nodes.extend(lines.and_then(|lines| card(at, lines)));
    nodes
}

fn rule(layout: LayoutStyle) -> NodeBuilder {
    container()
        .layout(layout.position(Position::Absolute))
        .class([Token::Gray8.background()])
}
