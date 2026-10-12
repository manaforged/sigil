use crate::events::{NodeEventsExt, PointerEvent};
use crate::kernel::{NodeBuilder, container};
use crate::layout::{LayoutBuilderExt, LayoutReactiveExt, LayoutStyle};
use crate::reactive::{Signal, derived, signal};

use super::super::scroll::{
    SCROLLBAR_WIDTH, THUMB_ATOMS, THUMB_MIN_HEIGHT, TRACK_ATOMS, geometry, to_i16,
};
use super::columns::ColWidth;
use super::state::TableState;

#[derive(Clone, Copy)]
pub(super) struct HScroll {
    pub scroll_x: Signal<f32>,
    pub max: Signal<f32>,
    pub extent: Signal<f32>,
    pub viewport: Signal<f32>,
    pub probe: Option<Signal<(f32, f32)>>,
}

pub(super) fn bind<Row: 'static>(
    state: &TableState<Row>,
    explicit: Option<f32>,
    prefix: f32,
) -> HScroll {
    let (viewport, probe) = explicit.map_or_else(
        || {
            let probe = signal((0.0_f32, 0.0_f32));
            (derived(move || probe.get().0), Some(probe))
        },
        |px| (signal(px), None),
    );
    let extent_state = state.clone();
    let extent = derived(move || prefix + extent_state.content_extent());
    HScroll {
        scroll_x: signal(0.0),
        max: derived(move || (extent.get() - viewport.get()).max(0.0)),
        extent,
        viewport,
        probe,
    }
}

impl HScroll {
    pub(super) fn shifted(&self, node: NodeBuilder) -> NodeBuilder {
        let hs = *self;
        let shift = derived(move || {
            (
                to_i16(hs.scroll_x.get().clamp(0.0, hs.max.get())),
                to_i16(hs.extent.get()),
            )
        });
        node.style_reactive(move || geometry::track_atoms(shift.get()))
    }

    pub(super) fn scrollbar(&self) -> NodeBuilder {
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
}

pub(super) fn probe(probe: Signal<(f32, f32)>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fill_width().height(0.0))
        .size_reactive(probe)
}

fn track(hs: HScroll) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fill_width().height(SCROLLBAR_WIDTH))
        .class(TRACK_ATOMS)
        .child(thumb(hs))
}

fn thumb(hs: HScroll) -> NodeBuilder {
    let geometry_signal = derived(move || {
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

fn drag(hs: HScroll, anchor: Signal<Option<(f32, f32)>>, ev: &mut PointerEvent) {
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

#[cfg(test)]
#[path = "hscroll_tests.rs"]
mod tests;

impl<Row: 'static> TableState<Row> {
    fn content_extent(&self) -> f32 {
        let layout = self.column_layout().get();
        self.visible_columns()
            .into_iter()
            .fold(0.0, |total, (_, column)| {
                total
                    + match (layout.widths.get(column.id()), column.col_width()) {
                        (Some(&px), _) => px.max(column.min_px()),
                        (None, ColWidth::Fixed(px)) => px.max(0.0),
                        (None, ColWidth::Fraction(_)) => column.min_px().max(0.0),
                    }
            })
    }
}
