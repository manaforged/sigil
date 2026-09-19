use crate::hal::{HitPosition, Rect};
use crate::kernel::{NodeBuilder, container, fragment};
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle, Position};
use crate::layout::with_text_geometry;
use crate::style::{ColorAtom, NodeStyleExt, Rgba8, StyleAtom};

use super::super::handlers::geometry_width;
use super::TextEditRuntime;

static SELECTION_ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundColor(ColorAtom::Rgba8(Rgba8 {
    r: 56,
    g: 118,
    b: 224,
    a: 130,
}))];

pub(super) fn selection_layer(runtime: &TextEditRuntime) -> NodeBuilder {
    let runtime = runtime.clone();
    fragment()
        .layout(LayoutStyle::block())
        .children_reactive(move || selection_rects(&runtime))
}

fn selection_rects(runtime: &TextEditRuntime) -> Vec<NodeBuilder> {
    let buffer = runtime.value.get();
    let Some(anchor) = runtime.selection.get() else {
        return Vec::new();
    };
    let caret = runtime.caret.get().min(buffer.len());
    let anchor = anchor.min(buffer.len());
    if anchor == caret {
        return Vec::new();
    }
    let style = runtime.text_style.get();
    let width = geometry_width(runtime);
    let anchor_position = HitPosition {
        byte_index: anchor,
        affinity: runtime.anchor_affinity.get(),
    };
    let focus_position = HitPosition {
        byte_index: caret,
        affinity: runtime.caret_affinity.get(),
    };
    let rects = with_text_geometry(|engine| {
        engine.selection_rects(&buffer, &style, width, (anchor_position, focus_position))
    })
    .unwrap_or_default();
    rects.into_iter().map(selection_rect).collect()
}

fn selection_rect(rect: Rect) -> NodeBuilder {
    container()
        .hittable(false)
        .layout(
            LayoutStyle::block()
                .position(Position::Absolute)
                .inset_left(rect.origin.x)
                .inset_top(rect.origin.y)
                .width(rect.width)
                .height(rect.height),
        )
        .class(SELECTION_ATOMS)
}
