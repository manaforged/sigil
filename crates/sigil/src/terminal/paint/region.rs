use std::cell::RefCell;
use std::ops::Range;

use crate::geometry::Rect;
use crate::profile::{Counter, count_add};
use crate::scene::CommandScene;
use crate::terminal::glyphs::Glyphs;

use super::mask::{Mask, Window};
use super::scene::{Paint, walk};
use super::{Cell, Grid, floor_cell, map_glyph};

const GROW: usize = 1;
const REACH: f32 = 3.0;

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::default());
}

#[derive(Default)]
struct Scratch {
    grid: Option<Grid>,
    reach: Vec<Rect>,
    windows: Mask,
    spans: Vec<Range<usize>>,
}

pub(crate) fn raster_regions(
    grid: &mut Grid,
    scene: &CommandScene,
    paint: &Paint<'_>,
    regions: &[Rect],
) {
    if regions.is_empty() {
        return;
    }
    SCRATCH.with(|scratch| {
        let copied = scratch.borrow_mut().raster(grid, scene, paint, regions);
        count_add(Counter::TuiCellsRastered, copied);
    });
}

impl Scratch {
    fn raster(
        &mut self,
        grid: &mut Grid,
        scene: &CommandScene,
        paint: &Paint<'_>,
        regions: &[Rect],
    ) -> u64 {
        let cell = if paint.cell <= 0.0 { 1.0 } else { paint.cell };
        self.reach.clear();
        self.reach
            .extend(regions.iter().map(|region| region.expand(REACH * cell)));
        self.windows
            .set(regions.iter().map(|region| window(grid, *region, cell)));
        let mut fresh = self.grid.take().unwrap_or_else(|| Grid::new(0, 0));
        let mut mask = fresh.mask.take().unwrap_or_default();
        mask.set(self.reach.iter().map(|rect| window(grid, *rect, cell)));
        prepare(&mut fresh, grid, mask, &mut self.spans);
        walk(&mut fresh, scene, paint, Some(&self.reach));
        let copied = self.copy(grid, &fresh, paint.caps.glyphs);
        std::mem::swap(&mut grid.images, &mut fresh.images);
        grid.cursor = fresh.cursor;
        self.grid = Some(fresh);
        copied
    }

    fn copy(&mut self, grid: &mut Grid, fresh: &Grid, glyphs: Glyphs) -> u64 {
        let mut copied = 0_u64;
        for y in self.windows.rows() {
            self.windows.spans(y, &mut self.spans);
            for span in &self.spans {
                for x in span.clone() {
                    copied += u64::from(copy_cell(grid, fresh, (x, y), glyphs));
                }
            }
        }
        copied
    }
}

fn prepare(fresh: &mut Grid, grid: &Grid, mask: Mask, spans: &mut Vec<Range<usize>>) {
    if (fresh.cols, fresh.rows) != (grid.cols, grid.rows) {
        fresh.reshape(grid.cols, grid.rows, Cell::blank());
    }
    fresh.graphics = grid.graphics;
    fresh.images.clear();
    fresh.cursor = None;
    for y in mask.rows() {
        mask.spans(y, spans);
        let start = y.saturating_mul(fresh.cols);
        for span in spans.iter() {
            let range = start.saturating_add(span.start)..start.saturating_add(span.end);
            if let Some(cells) = fresh.cells.get_mut(range.clone()) {
                cells.fill(Cell::blank());
            }
            if let Some(edges) = fresh.edges.get_mut(range) {
                edges.fill(None);
            }
        }
    }
    fresh.mask = Some(mask);
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

fn copy_cell(grid: &mut Grid, fresh: &Grid, (x, y): (usize, usize), glyphs: Glyphs) -> bool {
    let Some(index) = grid.index(x, y) else {
        return false;
    };
    let copied =
        if let (Some(cell), Some(next)) = (grid.cells.get_mut(index), fresh.cells.get(index)) {
            cell.clone_from(next);
            if glyphs != Glyphs::Unicode {
                map_glyph(cell, glyphs);
            }
            true
        } else {
            false
        };
    if let (Some(edge), Some(next)) = (grid.edges.get_mut(index), fresh.edges.get(index)) {
        *edge = *next;
    }
    copied
}

#[cfg(test)]
#[path = "region_tests.rs"]
mod tests;
