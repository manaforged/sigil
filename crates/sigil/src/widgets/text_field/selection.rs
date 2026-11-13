use std::borrow::Cow;

use crate::events::DragPhase;
use crate::geometry::Point;
use crate::layout::{LayoutStyle, Overflow, Position};
use crate::node::{NodeBuilder, TextBuilder, container, text};
use crate::reactive::{Signal, memo};
use crate::style::{Paint, Token};
use crate::text::{HitPosition, measure_text};
use crate::widgets::Widget;
use crate::widgets::text_edit::{blink, overlay_rect};

use super::handlers::TextFieldRuntime;

const CARET_W: f32 = 2.0;
const LINE_BOX: &str = "\u{200b}";

pub(super) fn text_region(runtime: TextFieldRuntime, displayed: Signal<String>) -> NodeBuilder {
    let region = container()
        .layout(
            LayoutStyle::block()
                .position(Position::Relative)
                .fill_width()
                .overflow_x(Overflow::Hidden),
        )
        .child(line_box())
        .child(track(runtime.clone(), displayed));
    region.pointer_drag(move |phase, event| match phase {
        DragPhase::Start => {
            if runtime.disabled() {
                event.prevent_default();
                return;
            }
            let byte = runtime.hit(event.position.x);
            runtime.state.select(byte);
            runtime.commit_caret();
        }
        DragPhase::Move => {
            if runtime.disabled() {
                return;
            }
            let byte = runtime.hit(event.position.x);
            runtime.state.extend(byte);
            runtime.commit_caret();
            event.stop_propagation();
        }
        DragPhase::End | DragPhase::Cancel => {}
    })
}

fn line_box() -> TextBuilder {
    text(LINE_BOX).hittable(false).layout(LayoutStyle::block())
}

fn track(runtime: TextFieldRuntime, displayed: Signal<String>) -> NodeBuilder {
    let scroll = runtime.scroll_x;
    let track_pos = memo(move || (-scroll.get(), 0.0));
    let (val_sig, preedit_sig) = (runtime.state.value(), runtime.preedit);
    let placeholder_visible =
        memo(move || val_sig.get().is_empty() && preedit_sig.get().is_empty());
    container()
        .hittable(false)
        .layout(LayoutStyle::block())
        .position_reactive(track_pos)
        .child({
            let runtime = runtime.clone();
            overlay_layer().children_reactive(move || selection_rects(&runtime))
        })
        .child({
            let (value, caret) = (runtime.state.value(), runtime.state.caret());
            let lit = blink(runtime.focused, move || {
                value.with(|_| ());
                caret.with(|_| ());
            });
            overlay_layer().children_reactive(move || {
                if lit.get() {
                    caret_rects(&runtime)
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
        .text_color(Token::OnSurface)
        .paint_when(placeholder_visible, |s| s.text_color(Token::OnSurfaceMuted))
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
    let Some(range) = runtime.state.selection() else {
        return Vec::new();
    };
    let value = runtime.state.value().get();
    let (layout, lo, hi) = runtime.layout_span(&value, range.start, range.end);
    let style = runtime.text_style.get();
    let span = measure_text(|engine| {
        let at = |byte| engine.caret(&layout, &style, None, HitPosition::downstream(byte));
        (at(lo), at(hi))
    });
    let Some((start, end)) = span.filter(|(start, end)| end.x() > start.x()) else {
        return Vec::new();
    };
    vec![
        overlay_rect(start.x(), start.top(), end.x() - start.x(), start.height())
            .background(Token::Selection),
    ]
}

fn caret_rects(runtime: &TextFieldRuntime) -> Vec<NodeBuilder> {
    if !runtime.focused.get() || !runtime.caret_visible.get() {
        return Vec::new();
    }
    let value = runtime.state.value().get();
    let caret = runtime.state.caret().get();
    let (composed, cursor) = runtime.composing_caret(&value, caret);
    let hiding = runtime.hiding();
    let (layout, byte) = super::mask::layout_text(&composed, cursor, hiding);
    let style = runtime.text_style.get();
    let Some(pos) =
        measure_text(|engine| engine.caret(&layout, &style, None, HitPosition::downstream(byte)))
    else {
        return Vec::new();
    };
    vec![overlay_rect(pos.x(), pos.top(), CARET_W, pos.height()).background(Token::OnSurface)]
}

impl TextFieldRuntime {
    fn hit(&self, x: f32) -> usize {
        let value = self.state.value().peek();
        let point = Point::new(x + self.scroll_x.peek(), 0.0);
        let style = self.text_style.peek();
        let hiding = self.hiding_peek();
        let layout = if hiding {
            Cow::Owned(super::mask::mask_chars(&value))
        } else {
            Cow::Borrowed(value.as_str())
        };
        let byte = measure_text(|engine| engine.hit(&layout, &style, None, point))
            .map_or(0, |hit| hit.byte_index());
        if hiding {
            super::mask::unmask_byte(&value, byte)
        } else {
            byte
        }
    }

    fn layout_span(&self, value: &str, lo: usize, hi: usize) -> (String, usize, usize) {
        if self.hiding() {
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
