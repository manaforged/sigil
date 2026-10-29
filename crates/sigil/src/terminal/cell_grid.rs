use unicode_segmentation::UnicodeSegmentation;

use crate::app::cell_width;
use crate::terminal::paint::{Cell, cell_text};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CellGrid {
    pub area: [u16; 4],
    pub cells: Vec<Cell>,
}

impl CellGrid {
    #[must_use]
    pub fn new(area: [u16; 4], fill: Cell) -> Self {
        Self {
            area,
            cells: vec![fill; usize::from(area[2]) * usize::from(area[3])],
        }
    }

    fn slot(&self, [x, y]: [u16; 2]) -> Option<usize> {
        let [left, top, width, height] = self.area;
        let (col, row) = (x.checked_sub(left)?, y.checked_sub(top)?);
        (col < width && row < height)
            .then(|| usize::from(row) * usize::from(width) + usize::from(col))
    }

    #[must_use]
    pub fn get(&self, at: [u16; 2]) -> Option<&Cell> {
        self.slot(at).and_then(|index| self.cells.get(index))
    }

    pub fn get_mut(&mut self, at: [u16; 2]) -> Option<&mut Cell> {
        self.slot(at).and_then(|index| self.cells.get_mut(index))
    }

    #[must_use]
    pub fn row(&self, index: u16) -> Option<&[Cell]> {
        let [_, _, width, height] = self.area;
        let start = usize::from(index) * usize::from(width);
        if index >= height {
            return None;
        }
        self.cells.get(start..start + usize::from(width))
    }

    pub fn write_text(&mut self, at: [u16; 2], text: &str, style: &Cell) -> u16 {
        let [mut x, y] = at;
        for grapheme in text.graphemes(true) {
            let step = u16::try_from(cell_width(grapheme)).unwrap_or(u16::MAX);
            if step == 0 {
                continue;
            }
            if (0..step).any(|pad| self.slot([x.saturating_add(pad), y]).is_none()) {
                break;
            }
            for pad in 0..step {
                if let Some(cell) = self.get_mut([x + pad, y]) {
                    *cell = Cell {
                        text: cell_text(grapheme, usize::from(pad)),
                        ..style.clone()
                    };
                }
            }
            x += step;
        }
        x - at[0]
    }
}

#[cfg(test)]
#[path = "cell_grid_tests.rs"]
mod tests;
