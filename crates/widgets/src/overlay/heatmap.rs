use sigil_kernel::{NodeBuilder, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_style::{NodeStyleExt, Rgba8, Style};

pub const CELL_PX: f32 = 8.0;

#[must_use]
pub fn heatmap<F: Fn(usize, usize) -> Rgba8>(
    grid_w: usize,
    grid_h: usize,
    color: F,
) -> NodeBuilder {
    let mut root = container().layout(LayoutStyle::flex_column());
    for y in 0..grid_h {
        let mut row = container().layout(LayoutStyle::flex_row());
        for x in 0..grid_w {
            row = row.child(cell(color(x, y)));
        }
        root = root.child(row);
    }
    root
}

fn cell(color: Rgba8) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(CELL_PX, CELL_PX))
        .style(Style::new().bg(color))
}

#[cfg(test)]
#[path = "heatmap_tests.rs"]
mod tests;
