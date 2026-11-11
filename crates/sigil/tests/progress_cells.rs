#![cfg(feature = "terminal")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use sigil::kernel::{HostMode, container};
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use sigil::terminal::{Grid, Ink};

const CELLS: u16 = 8;

fn painted_toned(value: f32, tone: Option<Tone>) -> Grid {
    sigil::reactive::Owner::scope("progress-cells", || {
        let mut host = sigil::Host::new(HostMode::Cell);
        host.mount(move || {
            let bar = match tone {
                Some(tone) => progress(signal(value), 100.0).tone(tone),
                None => progress(signal(value), 100.0),
            };
            container()
                .layout(LayoutStyle::block().width(f32::from(CELLS) * sigil::Host::CELL))
                .child(bar)
        });
        drop(host.grid([], 20, 3).unwrap());
        host.grid([], 20, 3).unwrap()
    })
}

fn painted(value: f32) -> Grid {
    painted_toned(value, None)
}

fn row(grid: &Grid) -> Vec<(String, Ink)> {
    (0..usize::from(CELLS))
        .map(|col| {
            let cell = grid.get(col, 0).unwrap();
            (cell.text().to_owned(), cell.bg)
        })
        .collect()
}

fn bar(grid: &Grid, fill: Ink) -> String {
    row(grid)
        .iter()
        .map(|(glyph, ink)| match (glyph.as_str(), *ink) {
            (_, ink) if ink == fill => '█',
            ("─", _) => '─',
            (glyph, _) => glyph.chars().next().unwrap_or(' '),
        })
        .collect()
}

#[test]
fn a_terminal_progress_bar_fills_whole_cells_for_its_value() {
    let fill = row(&painted(100.0))[0].1;
    for (value, shown) in [
        (0.0, "────────"),
        (1.0, "────────"),
        (50.0, "████────"),
        (100.0, "████████"),
    ] {
        let grid = painted(value);
        assert_eq!(bar(&grid, fill), shown, "{value}%: {:?}", row(&grid));
    }
}
