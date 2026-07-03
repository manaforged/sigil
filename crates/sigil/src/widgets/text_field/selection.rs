//! Pointer hit-testing + caret/selection overlay for the single-line field.
//!
//! Caret and selection geometry come from the host text engine via
//! [`with_text_geometry`] — core widgets stay renderer-agnostic (DR-054), so
//! the engine is injected rather than depended on directly. When no provider is
//! installed (headless layout tests) the overlay is empty and pointer hits
//! resolve to byte 0; keyboard editing is unaffected.
//!
//! Layout is a `relative` region wrapping the text, with two `absolute` layers
//! whose children are rebuilt reactively (`children_reactive`) from the
//! buffer's mirrored caret/selection signals: the selection highlight paints
//! behind the glyphs, the caret in front.

use events::prelude::{NodeEventsExt, PointerEvent};
use kernel::{NodeBuilder, TextBuilder, container, text};
use layout::prelude::{LayoutBuilderExt, LayoutReactiveExt, LayoutStyle, Overflow, Position};
use layout::with_text_geometry;
use reactive::{Signal, derived};
use style::{ColorAtom, NodeStyleExt, Rgba8, StyleAtom, TextStyle};

use crate::interaction::InteractionState;

use super::handlers::{TextFieldRuntime, commit};

/// Caret paint width in logical px.
const CARET_W: f32 = 2.0;

/// Selection highlight — a translucent blue, painted behind the glyphs. The
/// console theme is monochrome (its `primary` is near-white), so a near-white
/// wash reads as "smudge", not "selection"; a tinted blue is the universally
/// recognised text-selection cue and stays legible under light text. A
/// product-level design constant, like the field's invalid-state red.
static SELECTION_ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundColor(ColorAtom::Rgba8(Rgba8 {
    r: 56,
    g: 118,
    b: 224,
    a: 130,
}))];

/// Caret colour — matches the field's text token (`text-on-surface`).
static CARET_ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-surface")];

/// Fallback line height (logical px) when no text-geometry provider is
/// installed (headless layout / golden tests). Matches the default text-cell
/// height so the viewport keeps the field's historic single-line size; real
/// runs read the caret height from the engine so the viewport matches the font.
const FALLBACK_LINE_H: f32 = 16.0;

/// Build the field's text region: a single-line *viewport* (fixed `column`
/// width, clipped) over a `track` that holds the glyphs plus reactive selection
/// and caret overlays. The track is absolutely positioned and translated left
/// by the runtime's horizontal `scroll` (in px) so the caret stays in view and
/// long text scrolls instead of wrapping. Drop-in replacement for the old bare
/// text block in `render`.
pub(super) fn text_region(
    runtime: TextFieldRuntime,
    interaction: InteractionState,
    displayed: Signal<String>,
    width: Option<f32>,
) -> NodeBuilder {
    let column = column_width(width);
    let region = container()
        .layout(
            LayoutStyle::block()
                .position(Position::Relative)
                .width(column)
                .height(line_height())
                // Clip overflow so a long line scrolls within the field
                // instead of painting over its neighbours (Send button, etc.).
                .overflow_x(Overflow::Hidden)
                .overflow_y(Overflow::Hidden),
        )
        .child(track(runtime.clone(), interaction, displayed));
    attach_pointer(region, runtime, interaction)
}

/// The scrolling inner track: absolutely positioned, its inset bound (leak-free)
/// to `(-scroll, 0)` so the whole text slides horizontally as the caret moves.
/// Sizes to its content (no width) so the glyphs lay out on one line — the
/// viewport above clips what spills past `column`.
fn track(
    runtime: TextFieldRuntime,
    interaction: InteractionState,
    displayed: Signal<String>,
) -> NodeBuilder {
    let scroll = runtime.scroll;
    let track_pos = derived(move || (-scroll.get(), 0.0));
    container()
        .hittable(false)
        .layout(LayoutStyle::block())
        .position_reactive(track_pos)
        .child(selection_layer(runtime.clone()))
        .child(field_text(displayed))
        .child(caret_layer(runtime, interaction))
}

pub(super) fn column_width(width: Option<f32>) -> f32 {
    width.map_or(156.0, |w| (w - 26.0).max(0.0))
}

/// Viewport line height — the caret height reported by the text engine for the
/// field's style, so the single-line clip box matches the font. Falls back to
/// [`FALLBACK_LINE_H`] when no geometry provider is installed.
fn line_height() -> f32 {
    with_text_geometry(|engine| engine.caret("", &TextStyle::default(), 0).height)
        .filter(|height| *height > 0.0)
        .unwrap_or(FALLBACK_LINE_H)
}

/// The text glyphs. Geometry is computed against `TextStyle::default()` because
/// the field applies no size/weight class (only `text-on-surface`); if that
/// changes, the overlay style must change in lockstep. No width constraint, so
/// the line never wraps — horizontal scroll + the viewport clip handle overflow.
///
/// Non-hittable so the wrapping region stays the hit-path leaf: the router
/// binds pointer capture to the leaf (`router::route_pointer`), and the drag
/// handlers live on the region — a hittable text node would steal the capture.
fn field_text(displayed: Signal<String>) -> TextBuilder {
    text(displayed)
        .hittable(false)
        .layout(LayoutStyle::block())
        .class("text-on-surface")
}

fn selection_layer(runtime: TextFieldRuntime) -> NodeBuilder {
    overlay_layer().children_reactive(move || selection_rects(&runtime))
}

fn caret_layer(runtime: TextFieldRuntime, interaction: InteractionState) -> NodeBuilder {
    overlay_layer().children_reactive(move || caret_rects(&runtime, interaction))
}

/// A full-region absolute layer that never intercepts pointer events.
fn overlay_layer() -> NodeBuilder {
    container().hittable(false).layout(
        LayoutStyle::block()
            .position(Position::Absolute)
            .inset_left(0.0)
            .inset_top(0.0),
    )
}

fn selection_rects(runtime: &TextFieldRuntime) -> Vec<NodeBuilder> {
    let value = runtime.value.get();
    let caret = runtime.caret.get();
    let Some(anchor) = runtime.selection.get() else {
        return Vec::new();
    };
    let (lo, hi) = if anchor <= caret {
        (anchor, caret)
    } else {
        (caret, anchor)
    };
    let geom = with_text_geometry(|engine| {
        let start = engine.caret(&value, &TextStyle::default(), lo);
        let end = engine.caret(&value, &TextStyle::default(), hi);
        (start.x, end.x, start.top, start.height)
    });
    let Some((x0, x1, top, height)) = geom else {
        return Vec::new();
    };
    let width = x1 - x0;
    if width <= 0.0 {
        return Vec::new();
    }
    vec![rect(x0, top, width, height).class(SELECTION_ATOMS)]
}

fn caret_rects(runtime: &TextFieldRuntime, interaction: InteractionState) -> Vec<NodeBuilder> {
    // Caret shows only while the field is focused and the blink is on-phase.
    if !interaction.focus_visible().get() || !runtime.caret_visible.get() {
        return Vec::new();
    }
    let value = runtime.value.get();
    let caret = runtime.caret.get();
    let geom = with_text_geometry(|engine| {
        let pos = engine.caret(&value, &TextStyle::default(), caret);
        (pos.x, pos.top, pos.height)
    });
    let Some((x, top, height)) = geom else {
        return Vec::new();
    };
    vec![rect(x, top, CARET_W, height).class(CARET_ATOMS)]
}

/// An absolutely-positioned highlight/caret rectangle within the region.
fn rect(x: f32, top: f32, width: f32, height: f32) -> NodeBuilder {
    container().hittable(false).layout(
        LayoutStyle::block()
            .position(Position::Absolute)
            .inset_left(x)
            .inset_top(top)
            .width(width)
            .height(height),
    )
}

fn attach_pointer(
    node: NodeBuilder,
    runtime: TextFieldRuntime,
    interaction: InteractionState,
) -> NodeBuilder {
    let down = runtime.clone();
    let moved = runtime;
    node.pointer_down(move |event| pointer_down(event, &down))
        .pointer_move(move |event| pointer_move(event, &moved))
        .pointer_up(move |event| pointer_up(event, interaction))
}

fn pointer_down(event: &mut PointerEvent, runtime: &TextFieldRuntime) {
    if runtime.disabled() {
        return;
    }
    let byte = hit(runtime, event.position.x);
    let mut state = runtime.edit.borrow_mut();
    state.place_caret(byte);
    commit(runtime, &state);
    drop(state);
    // Capture so a drag that leaves the text still routes here. Do NOT stop
    // propagation: the click must bubble to the field for focus + press state.
    event.capture_pointer();
}

fn pointer_move(event: &mut PointerEvent, runtime: &TextFieldRuntime) {
    if !event.is_captured() || runtime.disabled() {
        return;
    }
    let byte = hit(runtime, event.position.x);
    let mut state = runtime.edit.borrow_mut();
    state.extend_to(byte);
    commit(runtime, &state);
    drop(state);
    event.stop_propagation();
}

fn pointer_up(event: &mut PointerEvent, interaction: InteractionState) {
    if event.is_captured() {
        // A captured pointer_up never reaches the outer field handler, so
        // release the press state here (see pointer_dispatch capture path).
        let _ = interaction.release_pointer(event);
    }
}

fn hit(runtime: &TextFieldRuntime, x: f32) -> usize {
    let value = runtime.value.peek();
    // `x` is region-local; the text track is translated left by `scroll`, so
    // the position in text space is `x + scroll`. At rest (scroll 0) this is a
    // no-op — clicks land exactly where the glyphs are.
    let text_x = x + runtime.scroll.peek();
    with_text_geometry(|engine| engine.hit(&value, &TextStyle::default(), text_x)).unwrap_or(0)
}
