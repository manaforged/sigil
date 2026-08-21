//! Heatmap debug overlay widget.
//!
//! Eagerly materialises a `grid_w * grid_h` grid of cells. Each cell is
//! a fixed-size block container with a concrete background colour
//! sampled from the caller's `color` closure at build time. Cells are
//! laid out left-to-right inside per-row flex containers, which are
//! stacked top-to-bottom inside the outer column container.
//!
//! No per-frame closure storage — the colour function is consumed
//! eagerly during the call to [`heatmap`].

use sigil_kernel::{NodeBuilder, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_style::{NodeStyleExt, Rgba8, Style};

/// Cell edge length in logical pixels.
pub const CELL_PX: f32 = 8.0;

/// Build a heatmap debug overlay.
///
/// `color(x, y)` is evaluated eagerly for every cell at build time and
/// the result is baked into a leaf container's background colour. The
/// closure is not retained past this call.
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
