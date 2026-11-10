use crate::reactive::derived;

use super::columns::ColWidth;
use super::state::TableState;

pub(super) use crate::widgets::scroll::Horizontal as HScroll;
pub(super) use crate::widgets::scroll::horizontal_probe as probe;

pub(super) fn bind<Row: 'static>(
    state: &TableState<Row>,
    explicit: Option<f32>,
    prefix: f32,
) -> HScroll {
    let extent_state = state.clone();
    let extent = derived(move || prefix + extent_state.content_extent());
    HScroll::new(extent, explicit)
}

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

#[cfg(test)]
#[path = "hscroll_tests.rs"]
mod tests;
