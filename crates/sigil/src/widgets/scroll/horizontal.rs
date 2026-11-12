use crate::events::{Key, KeyEvent, PointerEvent};
use crate::kernel::{NodeBuilder, container};
use crate::layout::{LayoutStyle, Overflow};
use crate::reactive::{Signal, memo, signal};
use crate::widgets::{Layout, Widget};

use super::{
    SCROLL_LINE, SCROLLBAR_WIDTH, THUMB_ATOMS, THUMB_MIN_HEIGHT, TRACK_ATOMS, WheelAxis, geometry,
    to_i16, wheel,
};

#[derive(Clone, Copy)]
pub(crate) struct Horizontal {
    pub(crate) scroll_x: Signal<f32>,
    pub(crate) max: Signal<f32>,
    pub(crate) extent: Signal<f32>,
    pub(crate) viewport: Signal<f32>,
    pub(crate) probe: Option<Signal<(f32, f32)>>,
}

impl Horizontal {
    pub(crate) fn new(extent: Signal<f32>, explicit: Option<f32>) -> Self {
        let (viewport, probe) = explicit.map_or_else(
            || {
                let probe = signal((0.0_f32, 0.0_f32));
                (memo(move || probe.get().0), Some(probe))
            },
            |px| (signal(px), None),
        );
        Self {
            scroll_x: signal(0.0),
            max: memo(move || (extent.get() - viewport.get()).max(0.0)),
            extent,
            viewport,
            probe,
        }
    }

    pub(crate) fn shifted(&self, node: NodeBuilder) -> NodeBuilder {
        let hs = *self;
        let shift = memo(move || {
            (
                to_i16(hs.scroll_x.get().clamp(0.0, hs.max.get())),
                to_i16(hs.extent.get()),
            )
        });
        node.style_reactive(move || geometry::track_atoms(shift.get()))
    }

    pub(crate) fn scrollbar(&self) -> NodeBuilder {
        let hs = *self;
        container()
            .layout(LayoutStyle::block().fill_width().shrink(0.0))
            .children_reactive(move || {
                if hs.max.get() <= 0.0 {
                    return Vec::new();
                }
                vec![move || track(hs)]
            })
    }

    pub(crate) fn wrap(self, content: NodeBuilder) -> NodeBuilder {
        let hs = self;
        let scrollable = memo(move || hs.max.get() > 0.0);
        let mut root = container()
            .layout(
                LayoutStyle::flex_column()
                    .fill_width()
                    .overflow_x(Overflow::Hidden),
            )
            .focusable(true)
            .mutate(move |node| node.a11y.tab_stop = Some(scrollable))
            .child(hs.shifted(content))
            .child(hs.scrollbar());
        if let Some(measure) = hs.probe {
            root = root.child(probe(measure));
        }
        root.key(Key::ArrowLeft, move |event| step(hs, event, -SCROLL_LINE))
            .key(Key::ArrowRight, move |event| step(hs, event, SCROLL_LINE))
            .scroll(move |event| {
                wheel(
                    event,
                    None,
                    Some(WheelAxis {
                        offset: hs.scroll_x,
                        viewport: hs.viewport.peek(),
                        max: hs.max.peek(),
                    }),
                );
            })
    }
}

fn step(hs: Horizontal, event: &mut KeyEvent, delta: f32) {
    let current = hs.scroll_x.peek();
    let next = (current + delta).clamp(0.0, hs.max.peek());
    if (next - current).abs() > f32::EPSILON {
        hs.scroll_x.set(next);
        event.stop_propagation();
    }
}

pub(crate) fn probe(probe: Signal<(f32, f32)>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fill_width().height(0.0))
        .size_reactive(probe)
}

fn track(hs: Horizontal) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fill_width().height(SCROLLBAR_WIDTH))
        .class(TRACK_ATOMS)
        .child(thumb(hs))
}

fn thumb(hs: Horizontal) -> NodeBuilder {
    let geometry_signal = memo(move || {
        let (width, travel) = geometry::thumb_metrics(hs.extent.get(), hs.viewport.get());
        let max = hs.max.get();
        let t = if max > 0.0 {
            hs.scroll_x.get().clamp(0.0, max) / max
        } else {
            0.0
        };
        (to_i16(width), to_i16(t * travel))
    });
    let drag_anchor = signal::<Option<(f32, f32)>>(None);
    let scroll_x = hs.scroll_x;
    container()
        .layout(LayoutStyle::block().fixed_size(THUMB_MIN_HEIGHT, SCROLLBAR_WIDTH))
        .class(THUMB_ATOMS)
        .style_reactive(move || geometry::thumb_atoms_x(geometry_signal.get()))
        .pointer_down(move |ev| {
            ev.capture_pointer();
            drag_anchor.set(Some((ev.position.x, scroll_x.peek())));
        })
        .pointer_up(move |_| drag_anchor.set(None))
        .pointer_move(move |ev| drag(hs, drag_anchor, ev))
}

fn drag(hs: Horizontal, anchor: Signal<Option<(f32, f32)>>, ev: &mut PointerEvent) {
    if !ev.is_captured() {
        return;
    }
    let Some((anchor_x, anchor_scroll)) = anchor.peek() else {
        return;
    };
    let max = hs.max.peek();
    let (_, travel) = geometry::thumb_metrics(hs.extent.peek(), hs.viewport.peek());
    if travel <= 0.0 || max <= 0.0 {
        return;
    }
    let dx = ev.position.x - anchor_x;
    hs.scroll_x
        .set(dx.mul_add(max / travel, anchor_scroll).clamp(0.0, max));
    ev.stop_propagation();
}
