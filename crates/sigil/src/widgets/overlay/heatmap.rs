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
mod tests {
    use super::*;
    use sigil_kernel::Cx;
    use sigil_reactive::Runtime;

    fn rgba(r: u8, g: u8, b: u8) -> Rgba8 {
        Rgba8 { r, g, b, a: 255 }
    }

    #[test]
    fn empty_grid_has_no_rows() {
        Runtime::scope("heatmap-empty", || {
            let mut cx = Cx::new_root();
            let root = cx.add(heatmap(0, 0, |_, _| rgba(0, 0, 0)));
            assert_eq!(cx.child_count(root), 0);
        });
    }

    #[test]
    fn grid_shape_matches_dimensions() {
        Runtime::scope("heatmap-shape", || {
            let mut cx = Cx::new_root();
            let root = cx.add(heatmap(3, 4, |_, _| rgba(0, 0, 0)));
            assert_eq!(cx.child_count(root), 4, "one child per row");
            for r in 0..4 {
                let row = cx.nth_child(root, r).expect("row exists");
                assert_eq!(cx.child_count(row), 3, "three cells per row");
            }
        });
    }

    #[test]
    fn closure_evaluated_eagerly_at_build() {
        Runtime::scope("heatmap-eager", || {
            use std::cell::Cell;
            let calls = Cell::new(0usize);
            let mut cx = Cx::new_root();
            let _ = cx.add(heatmap(5, 6, |_, _| {
                calls.set(calls.get() + 1);
                rgba(1, 2, 3)
            }));
            assert_eq!(
                calls.get(),
                30,
                "color() called once per cell at build time"
            );
        });
    }

    #[test]
    fn coordinates_cover_full_grid() {
        Runtime::scope("heatmap-coords", || {
            use std::cell::RefCell;
            let seen: RefCell<Vec<(usize, usize)>> = RefCell::new(Vec::new());
            let mut cx = Cx::new_root();
            let _ = cx.add(heatmap(2, 3, |x, y| {
                seen.borrow_mut().push((x, y));
                rgba(0, 0, 0)
            }));
            let mut seen = seen.into_inner();
            seen.sort_unstable();
            let mut expected: Vec<(usize, usize)> =
                (0..3).flat_map(|y| (0..2).map(move |x| (x, y))).collect();
            expected.sort_unstable();
            assert_eq!(seen, expected);
        });
    }
}
