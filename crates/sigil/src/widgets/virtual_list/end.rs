use std::cell::Cell;
use std::rc::Rc;

use crate::reactive::{Signal, effect, untrack};

use super::scroll::viewport_px;
use super::window::RowHeights;
use super::{BuildAxes, RowRender, VirtualListBuilder};

const EDGE_PX: f32 = 0.5;

#[derive(Clone)]
pub(crate) struct EndReached {
    rows: usize,
    run: Rc<dyn Fn()>,
}

impl EndReached {
    pub(crate) fn new(rows: usize, run: impl Fn() + 'static) -> Self {
        Self {
            rows,
            run: Rc::new(run),
        }
    }
}

impl<T, R> VirtualListBuilder<T, R>
where
    T: 'static,
    R: RowRender<T>,
{
    /// Runs `f` once each time the user scrolls within `rows` rows of the end, for load-more paging.
    ///
    /// ```
    /// use sigil::prelude::*;
    /// use std::rc::Rc;
    ///
    /// fn feed() -> impl IntoNode {
    ///     let items = signal(Rc::new((0..50).collect::<Vec<usize>>()));
    ///     virtual_list(items, |_, n: &usize| text(n.to_string())).end_reached(5, move || {
    ///         let next = items.peek().len() + 50;
    ///         items.set(Rc::new((0..next).collect()));
    ///     })
    /// }
    /// ```
    pub fn end_reached(self, rows: usize, f: impl Fn() + 'static) -> Self {
        self.end_hook(EndReached::new(rows, f))
    }

    pub(crate) fn end_hook(mut self, hook: EndReached) -> Self {
        self.end_reached = Some(hook);
        self
    }
}

fn last_shown(axes: BuildAxes, rows: usize) -> (usize, usize) {
    let bottom = axes.scroll_y.get() + viewport_px(axes.viewport.get(), axes.viewport_height);
    let at = |heights: RowHeights<'_>| (heights.rows(), heights.row_at(bottom - EDGE_PX));
    axes.tops.map_or_else(
        || at(RowHeights::fixed(rows, axes.row_height)),
        |tops| at(RowHeights::Tops(&tops.get())),
    )
}

pub(super) fn watch(axes: BuildAxes, live_rows: Signal<usize>, hook: EndReached) {
    let armed = Cell::new(true);
    let seen = Cell::new(usize::MAX);
    effect(move || {
        let (rows, last) = last_shown(axes, live_rows.get());
        if seen.replace(rows) != rows {
            armed.set(true);
        }
        let near = rows > 0 && last.saturating_add(hook.rows).saturating_add(1) >= rows;
        if !near {
            armed.set(true);
        } else if armed.replace(false) {
            untrack(|| (hook.run)());
        }
    });
}
