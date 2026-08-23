//! Horizontal scrolling for the `table()` view.
//!
//! One `scroll_x` signal drives the header track and every body row
//! track, so they shift in lockstep by construction. Rows and the
//! header keep their pre-scroll layout (`fill_width`, own clip); only
//! the inner cell track moves, by a reactive negative `MarginLeft`.
//! The extent is derived from the visible columns (`Fixed` pixels and
//! width overrides count in full, `Fraction` columns count their
//! min-width floor — exactly what flex resolves them to once fixed
//! columns overflow). When the total fits the viewport, `max` clamps
//! to zero, the offset stays 0, and no scrollbar mounts: identical
//! paint to a table without horizontal scrolling, including fraction
//! columns stretching to fill.
//!
//! ponytail: horizontal scrolling is table-scoped, not a general
//! two-axis `scroll()` widget. The `scroll()` widget clips explicit
//! content against a fixed extent; the table body is a `virtual_list`
//! with its own vertical windowing, and unifying the two architectures
//! is not justified by one consumer. Upgrade trigger: a second widget
//! needs horizontal scrolling.

use sigil_events::prelude::{NodeEventsExt, PointerEvent};
use sigil_kernel::{NodeBuilder, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutReactiveExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq, signal};
use sigil_style::NodeStyleExt;

use super::super::scroll::{
    SCROLLBAR_WIDTH, THUMB_ATOMS, THUMB_MIN_HEIGHT, TRACK_ATOMS, geometry, to_i16,
};
use super::columns::ColWidth;
use super::state::TableState;

/// Live horizontal-scroll signals shared by the header, body rows,
/// wheel handlers, and the bottom scrollbar.
#[derive(Clone, Copy)]
pub(super) struct HScroll {
    /// Content x offset in logical pixels, clamped to `[0, max]`.
    pub scroll_x: Signal<f32>,
    /// `(extent - viewport).max(0)` — zero while the content fits.
    pub max: Signal<f32>,
    /// Total width the visible columns want, in logical pixels.
    pub extent: Signal<f32>,
    /// Viewport width: explicit `viewport_width` or the measured
    /// content-box width published through [`HScroll::probe`].
    pub viewport: Signal<f32>,
    /// Width-probe target, present only when `viewport_width` was not
    /// set explicitly. Bind ONE node per signal (`SizeReactive`
    /// contract).
    pub probe: Option<Signal<(f32, f32)>>,
}

/// Build the horizontal-scroll signals for one table. `prefix` is the
/// pixel width of any fixed leading column (the opt-in checkbox
/// selection column); it counts into the extent in full.
pub(super) fn bind<Row: 'static>(
    state: &TableState<Row>,
    explicit: Option<f32>,
    prefix: f32,
) -> HScroll {
    let (viewport, probe) = explicit.map_or_else(
        || {
            let probe = signal((0.0_f32, 0.0_f32));
            (derived_eq(move || probe.get().0), Some(probe))
        },
        |px| (signal(px), None),
    );
    let extent_state = state.clone();
    let extent = derived_eq(move || prefix + content_extent(&extent_state));
    let max = derived_eq(move || (extent.get() - viewport.get()).max(0.0));
    HScroll {
        scroll_x: signal(0.0),
        max,
        extent,
        viewport,
        probe,
    }
}

/// Total width the columns want: full pixels for `Fixed` and overridden
/// columns, the min-width floor for `Fraction` columns (what flex
/// squeezes them to once the fixed columns overflow). Mirrors
/// [`super::columns::flex_cell`].
fn content_extent<Row: 'static>(state: &TableState<Row>) -> f32 {
    let layout = state.column_layout().get();
    let mut total = 0.0;
    for (_, column) in state.visible_columns() {
        total += match layout.widths.get(column.id()) {
            Some(&px) => px.max(column.min_px()),
            None => match column.col_width() {
                ColWidth::Fixed(px) => px.max(0.0),
                ColWidth::Fraction(_) => column.min_px().max(0.0),
            },
        };
    }
    total
}

impl HScroll {
    /// Shift `node` (a cell track) left by the clamped scroll offset and
    /// floor its width at the column extent. Never sets an explicit
    /// width: the track stays `fill_width`, so layout never depends on
    /// the (possibly not-yet-measured) viewport and a fitting table —
    /// including fraction columns stretching to fill — renders exactly
    /// as one without horizontal scrolling.
    pub(super) fn shifted(&self, node: NodeBuilder) -> NodeBuilder {
        let hs = *self;
        let shift = derived_eq(move || {
            (
                to_i16(hs.scroll_x.get().clamp(0.0, hs.max.get())),
                to_i16(hs.extent.get()),
            )
        });
        node.style_reactive(move || geometry::track_atoms(shift.get()))
    }

    /// Bottom scrollbar host. Empty (zero layout cost) while the content
    /// fits; an 8 px track with a draggable thumb once columns overflow.
    /// Mirrors the vertical scrollbar's track/thumb tokens.
    pub(super) fn scrollbar(&self) -> NodeBuilder {
        let hs = *self;
        container().children_reactive(move || {
            if hs.max.get() <= 0.0 {
                return Vec::new();
            }
            vec![track(hs)]
        })
    }
}

/// Zero-height full-width probe publishing the table's content-box
/// width. The root itself can't be measured: its 1 px border would
/// inflate the viewport by 2 px and phantom-enable scrolling.
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
    let geometry_signal = derived_eq(move || {
        let (width, travel) = geometry::thumb_metrics(hs.extent.get(), hs.viewport.get());
        let max = hs.max.get();
        let t = if max > 0.0 {
            hs.scroll_x.get().clamp(0.0, max) / max
        } else {
            0.0
        };
        (to_i16(width), to_i16(t * travel))
    });
    // Track-relative drag, mirroring the vertical thumb: anchor the
    // down-position and scroll value, translate moves via the
    // thumb-to-content ratio.
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
        .pointer_up(move |_| {
            drag_anchor.set(None);
        })
        .pointer_move(move |ev| {
            drag(hs, drag_anchor, ev);
        })
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
