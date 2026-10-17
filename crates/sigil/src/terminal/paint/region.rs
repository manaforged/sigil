use crate::geometry::Rect;
use crate::profile::{Counter, count_add};
use crate::scene::CommandScene;
use crate::terminal::glyphs::Glyphs;

use super::scene::{Paint, walk};
use super::{Cell, Grid, floor_cell, map_glyph};

const GROW: usize = 1;
const REACH: f32 = 3.0;

type Window = ((usize, usize), (usize, usize));

pub(crate) fn raster_regions(
    grid: &mut Grid,
    scene: &CommandScene,
    paint: &Paint<'_>,
    regions: &[Rect],
) {
    if regions.is_empty() {
        return;
    }
    let cell = if paint.cell <= 0.0 { 1.0 } else { paint.cell };
    let reach: Vec<Rect> = regions
        .iter()
        .map(|region| region.expand(REACH * cell))
        .collect();
    let mut scratch = blank_like(grid);
    walk(&mut scratch, scene, paint, Some(&reach));
    let mut copied = 0_u64;
    for region in regions {
        copied += copy(
            grid,
            &scratch,
            window(grid, *region, cell),
            paint.caps.glyphs,
        );
    }
    grid.images = scratch.images;
    grid.cursor = scratch.cursor;
    count_add(Counter::TuiCellsRastered, copied);
}

fn blank_like(grid: &Grid) -> Grid {
    let count = grid.cols.saturating_mul(grid.rows);
    Grid {
        cols: grid.cols,
        rows: grid.rows,
        cells: vec![Cell::blank(); count],
        edges: vec![None; count],
        images: Vec::new(),
        graphics: grid.graphics,
        cursor: None,
    }
}

fn window(grid: &Grid, region: Rect, cell: f32) -> Window {
    let first = |start: f32| floor_cell(start / cell).saturating_sub(GROW);
    let end = |stop: f32, limit: usize| floor_cell(stop / cell).saturating_add(1 + GROW).min(limit);
    (
        (first(region.origin.x), first(region.origin.y)),
        (
            end(region.origin.x + region.width, grid.cols),
            end(region.origin.y + region.height, grid.rows),
        ),
    )
}

fn copy(grid: &mut Grid, scratch: &Grid, ((x0, y0), (x1, y1)): Window, glyphs: Glyphs) -> u64 {
    let mut copied = 0_u64;
    for y in y0..y1 {
        for x in x0..x1 {
            let Some(index) = grid.index(x, y) else {
                continue;
            };
            let cells = (grid.cells.get_mut(index), scratch.cells.get(index));
            if let (Some(cell), Some(fresh)) = cells {
                cell.clone_from(fresh);
                if glyphs != Glyphs::Unicode {
                    map_glyph(cell, glyphs);
                }
                copied += 1;
            }
            let edges = (grid.edges.get_mut(index), scratch.edges.get(index));
            if let (Some(edge), Some(fresh)) = edges {
                *edge = *fresh;
            }
        }
    }
    copied
}

#[cfg(test)]
#[path = "region_tests.rs"]
mod tests;
