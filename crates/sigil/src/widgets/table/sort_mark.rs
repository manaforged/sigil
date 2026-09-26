use crate::kernel::{NodeBuilder, container};
use crate::reactive::{Signal, derived};
use crate::style::NodeStyleExt;

use super::state::{SortDir, TableState};

#[cfg(feature = "scene")]
static IDLE: &[crate::style::StyleAtom] = &[crate::style::StyleAtom::Opacity(3_000)];

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortMark {
    None,
    Idle,
    Ascending,
    Descending,
}

fn sort_state<Row: 'static>(
    state: &TableState<Row>,
    column: &'static str,
    sortable: bool,
) -> Signal<SortMark> {
    let sort = state.sort_key();
    derived(move || match sort.get() {
        Some(key) if key.column == column => match key.dir {
            SortDir::Ascending => SortMark::Ascending,
            SortDir::Descending => SortMark::Descending,
        },
        _ if sortable => SortMark::Idle,
        _ => SortMark::None,
    })
}

#[cfg(feature = "scene")]
pub(super) fn sort_mark<Row: 'static>(
    state: &TableState<Row>,
    column: &'static str,
    sortable: bool,
) -> NodeBuilder {
    use crate::icons::builtin::{ARROW_DOWN_SVG, ARROW_UP_SVG, CHEVRONS_UP_DOWN_SVG};
    use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle};
    use crate::style::space;
    use crate::widgets::icon_svg;
    let mark = sort_state(state, column, sortable);
    container().hittable(false).children_reactive(move || {
        let current = mark.get();
        let icon = match current {
            SortMark::None => return Vec::new(),
            SortMark::Idle => icon_svg(CHEVRONS_UP_DOWN_SVG),
            SortMark::Ascending => icon_svg(ARROW_UP_SVG).strong(),
            SortMark::Descending => icon_svg(ARROW_DOWN_SVG).strong(),
        };
        let slot = container()
            .layout(LayoutStyle::flex_row().padding_left(space(1)).shrink(0.0))
            .child(icon.px(12.0));
        vec![if current == SortMark::Idle {
            slot.class(IDLE)
        } else {
            slot
        }]
    })
}

#[cfg(not(feature = "scene"))]
pub(super) fn sort_mark<Row: 'static>(
    state: &TableState<Row>,
    column: &'static str,
    sortable: bool,
) -> NodeBuilder {
    use crate::kernel::text;
    let mark = sort_state(state, column, sortable);
    container().child(
        text(derived(move || {
            match mark.get() {
                SortMark::Ascending => "▲",
                SortMark::Descending => "▼",
                SortMark::None | SortMark::Idle => "",
            }
            .to_owned()
        }))
        .class("text-xs")
        .class(super::header::HEADER_TEXT),
    )
}
