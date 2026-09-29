use crate::geometry::{Point, Rect};
use crate::kernel::NodeId;
use crate::scene::builder::SceneBuilder;
use crate::scene::command::{CellRect, SceneRectStyle};
use crate::style::{SELECTION_FILL, TextStyle};

pub(super) struct SelectedText<'a> {
    pub rect: Rect,
    pub text: &'a str,
    pub style: &'a TextStyle,
    pub wrap: bool,
    pub range: (usize, usize),
}

pub(super) fn push_selection(scene: &mut SceneBuilder, id: NodeId, selected: &SelectedText<'_>) {
    let rect = selected.rect;
    let width = selected.wrap.then_some(rect.width);
    let rects = crate::layout::with_text_geometry(|engine| {
        engine.selection_rects(
            selected.text,
            selected.style,
            width,
            (
                crate::events::selection::selection_hit(selected.range.0),
                crate::events::selection::selection_hit(selected.range.1),
            ),
        )
    })
    .unwrap_or_default();
    let style = SceneRectStyle::solid(SELECTION_FILL);
    for local in rects {
        let band = Rect::new(
            Point::new(
                rect.origin.x + local.origin.x,
                rect.origin.y + local.origin.y,
            ),
            local.width,
            local.height,
        );
        if let Some(band) = band.intersection(rect) {
            scene.draw_rect_for_node(id, band, style, CellRect::default());
        }
    }
}
