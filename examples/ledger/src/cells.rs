use data_table::{Field, FieldKind};
use sigil::prelude::*;
use sigil::widgets::patterns::tooltip;

use crate::state::Live;
use data_table::ledger::Row;

const GREEN: Rgba8 = Rgba8 {
    r: 0,
    g: 128,
    b: 0,
    a: 255,
};
const RED: Rgba8 = Rgba8 {
    r: 220,
    g: 0,
    b: 0,
    a: 255,
};
static UP: &[StyleAtom] = &[StyleAtom::TextColor(ColorAtom::Rgba8(GREEN))];
static DOWN: &[StyleAtom] = &[StyleAtom::TextColor(ColorAtom::Rgba8(RED))];

#[must_use]
pub fn cell(live: Live, row: &Row, field: &'static Field) -> NodeBuilder {
    match field.kind {
        FieldKind::LastDone => price(live, row),
        FieldKind::Symbol => container().child(tooltip(plain(row, field), row.quote.name.clone())),
        _ => plain(row, field),
    }
}

fn plain(row: &Row, field: &Field) -> NodeBuilder {
    let mut node = text(row.text(field.kind)).nowrap();
    if field.tint {
        node = node.style_reactive(if row.quote.up() { || UP } else { || DOWN });
    }
    container().child(node)
}

fn price(live: Live, row: &Row) -> NodeBuilder {
    let index = row.index;
    let slot = u32::try_from(index).unwrap_or(u32::MAX);
    let prev_close = row.quote.prev_close;
    let editing = live.editing;
    container().children_reactive(move || {
        if editing.get() == Some(slot) {
            vec![editor(live)]
        } else {
            vec![ticker(live, index, prev_close)]
        }
    })
}

fn ticker(live: Live, index: usize, prev_close: f64) -> NodeBuilder {
    let prices = live.prices;
    let value = move || prices.get().get(index).copied().unwrap_or(prev_close);
    container().child(
        text_live(move || format!("{:.3}", value()))
            .nowrap()
            .style_reactive(move || if value() > prev_close { UP } else { DOWN }),
    )
}

fn editor(live: Live) -> NodeBuilder {
    container().child(
        text_field(live.draft)
            .invalid_when(live.invalid())
            .focus_when(live.focus)
            .submit(move |text| live.commit(text))
            .width(72.0),
    )
}
