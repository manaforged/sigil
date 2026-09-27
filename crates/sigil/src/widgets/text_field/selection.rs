use crate::events::prelude::{NodeEventsExt, PointerEvent};
use crate::hal::{HitPosition, Point, TextAffinity};
use crate::kernel::{NodeBuilder, TextBuilder, container, text};
use crate::layout::prelude::{
    LayoutBuilderExt, LayoutReactiveExt, LayoutStyle, Overflow, Position,
};
use crate::layout::with_text_geometry;
use crate::reactive::{Signal, derived, derived_eq};
use crate::style::{ColorAtom, NodeStyleExt, SELECTION_FILL, StyleAtom, TextStyle, Token};

use crate::widgets::interaction::InteractionState;

use super::super::text_edit::commit;
use super::handlers::TextFieldRuntime;

const CARET_W: f32 = 2.0;

static SELECTION_ATOMS: &[StyleAtom] =
    &[StyleAtom::BackgroundColor(ColorAtom::Rgba8(SELECTION_FILL))];

static CARET_ATOMS: &[StyleAtom] = &[Token::OnSurface.bg()];
static FIELD_TEXT_ATOMS: &[StyleAtom] = &[Token::OnSurface.text()];
static PLACEHOLDER_TEXT_ATOMS: &[StyleAtom] = &[Token::OnSurfaceMuted.text()];

const FALLBACK_LINE_H: f32 = 16.0;

pub(super) fn text_region(
    runtime: TextFieldRuntime,
    interaction: InteractionState,
    displayed: Signal<String>,
) -> NodeBuilder {
    let region = container()
        .layout(
            LayoutStyle::block()
                .position(Position::Relative)
                .fill_width()
                .height(line_height())
                .overflow_x(Overflow::Hidden)
                .overflow_y(Overflow::Hidden),
        )
        .child(track(runtime.clone(), interaction, displayed));
    attach_pointer(region, runtime, interaction)
}

fn track(
    runtime: TextFieldRuntime,
    interaction: InteractionState,
    displayed: Signal<String>,
) -> NodeBuilder {
    let scroll = runtime.input.scroll_x;
    let track_pos = derived(move || (-scroll.get(), 0.0));
    let placeholder_visible = derived_eq(move || {
        runtime.input.value.get().is_empty() && runtime.input.ime.preedit.get().is_empty()
    });
    container()
        .hittable(false)
        .layout(LayoutStyle::block())
        .position_reactive(track_pos)
        .child(selection_layer(runtime.clone()))
        .child(caret_layer(runtime, interaction))
        .child(field_text(displayed, placeholder_visible))
}

pub(super) fn column_width(width: Option<f32>) -> f32 {
    width.map_or(156.0, |w| (w - 26.0).max(0.0))
}

pub(super) fn line_height() -> f32 {
    with_text_geometry(|engine| {
        engine
            .caret("", &TextStyle::default(), None, downstream(0))
            .height
    })
    .filter(|height| *height > 0.0)
    .unwrap_or(FALLBACK_LINE_H)
}

fn field_text(displayed: Signal<String>, placeholder_visible: Signal<bool>) -> TextBuilder {
    text(displayed)
        .hittable(false)
        .layout(LayoutStyle::block())
        .style_reactive(move || {
            if placeholder_visible.get() {
                PLACEHOLDER_TEXT_ATOMS
            } else {
                FIELD_TEXT_ATOMS
            }
        })
}

fn selection_layer(runtime: TextFieldRuntime) -> NodeBuilder {
    overlay_layer().children_reactive(move || selection_rects(&runtime))
}

fn caret_layer(runtime: TextFieldRuntime, interaction: InteractionState) -> NodeBuilder {
    overlay_layer().children_reactive(move || caret_rects(&runtime, interaction))
}

fn overlay_layer() -> NodeBuilder {
    container().hittable(false).layout(
        LayoutStyle::block()
            .position(Position::Absolute)
            .inset_left(0.0)
            .inset_top(0.0),
    )
}

fn selection_rects(runtime: &TextFieldRuntime) -> Vec<NodeBuilder> {
    let value = runtime.input.value.get();
    let caret = runtime.input.caret.get();
    let Some(anchor) = runtime.input.selection.get() else {
        return Vec::new();
    };
    let (lo, hi) = if anchor <= caret {
        (anchor, caret)
    } else {
        (caret, anchor)
    };
    let (layout, lo, hi) = layout_span(runtime, &value, lo, hi);
    let style = runtime.input.text_style.get();
    let geom = with_text_geometry(|engine| {
        let start = engine.caret(&layout, &style, None, downstream(lo));
        let end = engine.caret(&layout, &style, None, downstream(hi));
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
    if !interaction.focus_visible().get() || !runtime.input.caret_visible.get() {
        return Vec::new();
    }
    let value = runtime.input.value.get();
    let caret = runtime.input.caret.get();
    let (layout, caret, _) = layout_span(runtime, &value, caret, caret);
    let style = runtime.input.text_style.get();
    let geom = with_text_geometry(|engine| {
        let pos = engine.caret(&layout, &style, None, downstream(caret));
        (pos.x, pos.top, pos.height)
    });
    let Some((x, top, height)) = geom else {
        return Vec::new();
    };
    vec![rect(x, top, CARET_W, height).class(CARET_ATOMS)]
}

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

const fn downstream(byte: usize) -> HitPosition {
    HitPosition {
        byte_index: byte,
        affinity: TextAffinity::Downstream,
    }
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
    let mut state = runtime.edit_mut();
    state.place_caret(byte);
    commit(&runtime.input, &state);
    drop(state);
    event.capture_pointer();
}

fn pointer_move(event: &mut PointerEvent, runtime: &TextFieldRuntime) {
    if !event.is_captured() || runtime.disabled() {
        return;
    }
    let byte = hit(runtime, event.position.x);
    let mut state = runtime.edit_mut();
    state.extend_to(byte);
    commit(&runtime.input, &state);
    drop(state);
    event.stop_propagation();
}

fn pointer_up(event: &mut PointerEvent, interaction: InteractionState) {
    if event.is_captured() {
        let _ = interaction.release_pointer(event);
    }
}

fn hit(runtime: &TextFieldRuntime, x: f32) -> usize {
    let value = runtime.input.value.peek();
    let text_x = x + runtime.input.scroll_x.peek();
    let style = runtime.input.text_style.peek();
    let hiding = super::mask::hiding_peek(runtime.password, runtime.reveal);
    if hiding {
        let layout = super::mask::mask_chars(&value);
        let mask_byte =
            with_text_geometry(|engine| engine.hit(&layout, &style, None, Point::new(text_x, 0.0)))
                .map_or(0, |hit| hit.byte_index);
        super::mask::unmask_byte(&value, mask_byte)
    } else {
        with_text_geometry(|engine| engine.hit(&value, &style, None, Point::new(text_x, 0.0)))
            .map_or(0, |hit| hit.byte_index)
    }
}

fn layout_span(
    runtime: &TextFieldRuntime,
    value: &str,
    lo: usize,
    hi: usize,
) -> (String, usize, usize) {
    if super::mask::hiding(runtime.password, runtime.reveal) {
        (
            super::mask::mask_chars(value),
            super::mask::mask_byte(value, lo),
            super::mask::mask_byte(value, hi),
        )
    } else {
        (value.to_owned(), lo, hi)
    }
}
