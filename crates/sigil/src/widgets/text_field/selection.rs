use crate::events::PointerEvent;
use crate::geometry::Point;
use crate::hal::{HitPosition, TextAffinity};
use crate::kernel::{NodeBuilder, TextBuilder, container, text};
use crate::layout::with_text_geometry;
use crate::layout::{LayoutStyle, Overflow, Position};
use crate::reactive::{Signal, derived};
use crate::style::{ColorAtom, SELECTION_FILL, StyleAtom, Token};
use crate::widgets::{Layout, Widget};

use crate::widgets::interaction::{InteractionState, is_primary};

use super::super::text_edit::{blink, overlay_rect};
use super::handlers::TextFieldRuntime;

const CARET_W: f32 = 2.0;

static SELECTION_ATOMS: &[StyleAtom] =
    &[StyleAtom::BackgroundColor(ColorAtom::Color(SELECTION_FILL))];

static CARET_ATOMS: &[StyleAtom] = &[Token::OnSurface.background()];
static FIELD_TEXT_ATOMS: &[StyleAtom] = &[Token::OnSurface.text()];
static PLACEHOLDER_TEXT_ATOMS: &[StyleAtom] = &[Token::OnSurfaceMuted.text()];

const LINE_BOX: &str = "\u{200b}";

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
                .overflow_x(Overflow::Hidden),
        )
        .child(line_box())
        .child(track(runtime.clone(), interaction, displayed));
    let moved = runtime.clone();
    region
        .pointer_down(move |event| pointer_down(event, &runtime))
        .pointer_move(move |event| pointer_move(event, &moved))
        .pointer_up(move |event| {
            if event.is_captured() {
                let _ = interaction.release_pointer(event);
            }
        })
}

fn line_box() -> TextBuilder {
    text(LINE_BOX).hittable(false).layout(LayoutStyle::block())
}

fn track(
    runtime: TextFieldRuntime,
    interaction: InteractionState,
    displayed: Signal<String>,
) -> NodeBuilder {
    let scroll = runtime.input.scroll_x;
    let track_pos = derived(move || (-scroll.get(), 0.0));
    let placeholder_visible = derived(move || {
        runtime.input.value.get().is_empty() && runtime.input.ime.preedit.get().is_empty()
    });
    container()
        .hittable(false)
        .layout(LayoutStyle::block())
        .position_reactive(track_pos)
        .child({
            let runtime = runtime.clone();
            overlay_layer().children_reactive(move || selection_rects(&runtime))
        })
        .child({
            let (value, caret) = (runtime.input.value, runtime.input.caret);
            let lit = blink(interaction, move || {
                value.with(|_| ());
                caret.with(|_| ());
            });
            overlay_layer().children_reactive(move || {
                if lit.get() {
                    caret_rects(&runtime, interaction)
                } else {
                    Vec::new()
                }
            })
        })
        .child(field_text(displayed, placeholder_visible))
}

pub(super) fn column_width(width: Option<f32>) -> f32 {
    width.map_or(156.0, |w| (w - 26.0).max(0.0))
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
    let (layout, lo, hi) = runtime.layout_span(&value, anchor.min(caret), anchor.max(caret));
    let style = runtime.input.text_style.get();
    let span = with_text_geometry(|engine| {
        let at = |byte| engine.caret(&layout, &style, None, downstream(byte));
        (at(lo), at(hi))
    });
    let Some((start, end)) = span.filter(|(start, end)| end.x > start.x) else {
        return Vec::new();
    };
    vec![overlay_rect(start.x, start.top, end.x - start.x, start.height).class(SELECTION_ATOMS)]
}

fn caret_rects(runtime: &TextFieldRuntime, interaction: InteractionState) -> Vec<NodeBuilder> {
    if !interaction.focused().get() || !runtime.input.caret_visible.get() {
        return Vec::new();
    }
    let (composed, cursor) = runtime
        .input
        .ime
        .composing_caret(&runtime.input.value.get(), runtime.input.caret.get());
    let (layout, caret, _) = runtime.layout_span(&composed, cursor, cursor);
    let style = runtime.input.text_style.get();
    let pos = with_text_geometry(|engine| engine.caret(&layout, &style, None, downstream(caret)));
    let Some(pos) = pos else {
        return Vec::new();
    };
    vec![overlay_rect(pos.x, pos.top, CARET_W, pos.height).class(CARET_ATOMS)]
}

const fn downstream(byte: usize) -> HitPosition {
    HitPosition {
        byte_index: byte,
        affinity: TextAffinity::Downstream,
    }
}

fn pointer_down(event: &mut PointerEvent, runtime: &TextFieldRuntime) {
    if runtime.input.disabled() || !is_primary(event) {
        return;
    }
    let byte = runtime.hit(event.position.x);
    let mut state = runtime.input.edit_mut();
    state.place_caret(byte);
    runtime.input.commit(&state);
    event.capture_pointer();
}

fn pointer_move(event: &mut PointerEvent, runtime: &TextFieldRuntime) {
    if !event.is_captured() || runtime.input.disabled() {
        return;
    }
    let byte = runtime.hit(event.position.x);
    let mut state = runtime.input.edit_mut();
    state.extend_to(byte);
    runtime.input.commit(&state);
    event.stop_propagation();
}

impl TextFieldRuntime {
    fn hit(&self, x: f32) -> usize {
        let value = self.input.value.peek();
        let point = Point::new(x + self.input.scroll_x.peek(), 0.0);
        let style = self.input.text_style.peek();
        let hiding = super::mask::hiding_peek(self.password, self.reveal);
        let layout = if hiding {
            std::borrow::Cow::Owned(super::mask::mask_chars(&value))
        } else {
            std::borrow::Cow::Borrowed(value.as_str())
        };
        let byte = with_text_geometry(|engine| engine.hit(&layout, &style, None, point))
            .map_or(0, |hit| hit.byte_index);
        if hiding {
            super::mask::unmask_byte(&value, byte)
        } else {
            byte
        }
    }

    fn layout_span(&self, value: &str, lo: usize, hi: usize) -> (String, usize, usize) {
        if super::mask::hiding(self.password, self.reveal) {
            (
                super::mask::mask_chars(value),
                super::mask::mask_byte(value, lo),
                super::mask::mask_byte(value, hi),
            )
        } else {
            (value.to_owned(), lo, hi)
        }
    }
}
