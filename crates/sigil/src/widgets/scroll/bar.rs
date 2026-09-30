use crate::events::{NodeEventsExt, PointerEvent};
use crate::kernel::{NodeBuilder, container};
use crate::layout::{LayoutBuilderExt, LayoutReactiveExt, LayoutStyle, Position};
use crate::reactive::{Signal, derived, signal};
use crate::style::{DisplayKind, NodeStyleExt, Radius, StyleAtom, Token};

use super::{SCROLLBAR_WIDTH, THUMB_MIN_HEIGHT, THUMB_REST_WIDTH, geometry, overflows, to_i16};

pub(crate) static THUMB_ATOMS: &[StyleAtom] =
    &[Token::SurfacePressedStrong.bg(), Radius::Full.atom()];
pub(crate) static TRACK_ATOMS: &[StyleAtom] = &[Radius::Full.atom()];
static TRACK_HOVER: &[StyleAtom] = &[Radius::Full.atom(), Token::SurfaceSubtle.bg()];
static HIDDEN: &[StyleAtom] = &[StyleAtom::Display(DisplayKind::None)];

pub(crate) fn scrollbar(
    scroll_y: Signal<f32>,
    max_scroll: Signal<f32>,
    extent: Signal<f32>,
    viewport_height: Signal<f32>,
    fill_height: bool,
) -> NodeBuilder {
    let hovered = signal(false);
    let overflow = overflows(max_scroll);
    let thumb = scrollbar_thumb(scroll_y, max_scroll, extent, viewport_height, hovered)
        .style_reactive(move || if overflow.get() { &[] } else { HIDDEN });
    let page = move |ev: &mut PointerEvent| {
        let (thumb_h, max_travel) = geometry::thumb_metrics(extent.peek(), viewport_height.peek());
        let max = max_scroll.peek();
        let thumb_top = if max > 0.0 && max_travel > 0.0 {
            scroll_y.peek().clamp(0.0, max) / max * max_travel
        } else {
            0.0
        };
        if ev.position.y >= thumb_top && ev.position.y < thumb_top + thumb_h {
            return;
        }
        let dir = if ev.position.y < thumb_top { -1.0 } else { 1.0 };
        scroll_y.set((scroll_y.peek() + dir * viewport_height.peek()).clamp(0.0, max));
        ev.stop_propagation();
    };
    let layout = LayoutStyle::block().width(SCROLLBAR_WIDTH).shrink(0.0);
    container()
        .layout(if fill_height {
            layout.fill_height()
        } else {
            layout
        })
        .style_reactive(move || {
            if hovered.get() && overflow.get() {
                TRACK_HOVER
            } else {
                TRACK_ATOMS
            }
        })
        .pointer_enter(move |_| hovered.set(true))
        .pointer_leave(move |_| hovered.set(false))
        .pointer_down_capture(page)
        .child(thumb)
}

fn scrollbar_thumb(
    scroll_y: Signal<f32>,
    max_scroll: Signal<f32>,
    extent: Signal<f32>,
    viewport_height: Signal<f32>,
    hovered: Signal<bool>,
) -> NodeBuilder {
    let height =
        derived(move || to_i16(geometry::thumb_metrics(extent.get(), viewport_height.get()).0));
    let position = derived(move || {
        let (_, max_travel) = geometry::thumb_metrics(extent.get(), viewport_height.get());
        let max = max_scroll.get();
        let t = if max > 0.0 {
            scroll_y.get().clamp(0.0, max) / max
        } else {
            0.0
        };
        (0.0, (t * max_travel).round())
    });
    let drag_anchor = signal::<Option<(f32, f32)>>(None);
    let thick = derived(move || hovered.get() || drag_anchor.get().is_some());

    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(THUMB_REST_WIDTH, THUMB_MIN_HEIGHT)
                .position(Position::Absolute)
                .inset_left(0.0)
                .inset_top(0.0),
        )
        .class(THUMB_ATOMS)
        .position_reactive(position)
        .style_reactive(move || geometry::thumb_atoms(height.get(), thick.get()))
        .pointer_down(move |ev| {
            ev.capture_pointer();
            drag_anchor.set(Some((ev.position.y, scroll_y.peek())));
        })
        .pointer_up(move |_| {
            drag_anchor.set(None);
        })
        .pointer_move(move |ev| {
            if !ev.is_captured() {
                return;
            }
            let Some((anchor_y, anchor_scroll)) = drag_anchor.peek() else {
                return;
            };
            let max = max_scroll.peek();
            let (_, max_travel) = geometry::thumb_metrics(extent.peek(), viewport_height.peek());
            if max_travel <= 0.0 || max <= 0.0 {
                return;
            }
            let drag_dy = ev.position.y - anchor_y;
            let offset_dy = drag_dy * (max / max_travel);
            let next = (anchor_scroll + offset_dy).clamp(0.0, max);
            scroll_y.set(next);
            ev.stop_propagation();
        })
}
