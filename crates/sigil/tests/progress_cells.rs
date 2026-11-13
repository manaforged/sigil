#![cfg(feature = "terminal")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use sigil::kernel::{HostMode, container};
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use sigil::terminal::{Grid, Ink};

const CELLS: u16 = 8;

fn mount_bar(make: impl Fn() -> sigil::widgets::ProgressBuilder + 'static) -> Grid {
    sigil::reactive::Owner::scope("progress-cells", || {
        let mut host = sigil::Host::new(HostMode::Cell);
        host.mount(move || {
            container()
                .layout(LayoutStyle::block().width(f32::from(CELLS) * sigil::Host::CELL))
                .child(make())
        });
        drop(host.grid([], 20, 3).unwrap());
        host.grid([], 20, 3).unwrap()
    })
}

fn painted_toned(value: f32, tone: Option<Tone>) -> Grid {
    mount_bar(move || {
        tone.map_or_else(
            || progress(signal(value), 100.0),
            |tone| progress(signal(value), 100.0).tone(tone),
        )
    })
}

fn painted(value: f32) -> Grid {
    painted_toned(value, None)
}

fn painted_indeterminate(phase: f32) -> Grid {
    mount_bar(move || progress(signal(phase), 1.0).indeterminate())
}

fn row(grid: &Grid) -> Vec<(String, Ink)> {
    (0..usize::from(CELLS))
        .map(|col| {
            let cell = grid.get(col, 0).unwrap();
            (cell.text().to_owned(), cell.bg())
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

fn assert_clean_bar(
    grid: &Grid,
    fill_bg: Ink,
    track_bg: Ink,
    contiguous: bool,
    label: &str,
) -> usize {
    let width = usize::from(CELLS);
    let mut filled = 0;
    let mut seen_track = false;
    for col in 0..width {
        let cell = grid.get(col, 0).unwrap();
        if cell.bg() == fill_bg {
            assert!(
                !contiguous || !seen_track,
                "{label} col {col}: fill after track in {:?}",
                row(grid)
            );
            assert_eq!(
                cell.text(),
                " ",
                "{label} col {col}: rule drawn over fill: {cell:?}"
            );
            filled += 1;
        } else {
            seen_track = true;
            assert_eq!(
                cell.bg(),
                track_bg,
                "{label} col {col}: unexpected bg: {cell:?}"
            );
            assert_eq!(
                cell.text(),
                "─",
                "{label} col {col}: unexpected track glyph: {cell:?}"
            );
        }
    }
    for col in width..grid.cols() {
        let cell = grid.get(col, 0).unwrap();
        assert_eq!(
            cell,
            sigil::terminal::Cell::blank(),
            "{label} col {col}: bled past bar"
        );
    }
    filled
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

#[test]
fn toned_and_indeterminate_bars_keep_solid_fills_and_stay_in_bounds() {
    let track_bg = painted(0.0).get(usize::from(CELLS) - 1, 0).unwrap().bg();
    for tone in [None, Some(Tone::Primary), Some(Tone::Info)] {
        let fill_bg = painted_toned(100.0, tone).get(0, 0).unwrap().bg();
        assert_eq!(
            assert_clean_bar(
                &painted_toned(0.0, tone),
                fill_bg,
                track_bg,
                true,
                &format!("0% {tone:?}")
            ),
            0
        );
    }

    let info_fill = painted_toned(100.0, Some(Tone::Info))
        .get(0, 0)
        .unwrap()
        .bg();
    let mut prev = 0;
    for pct in 0..=100_u8 {
        let grid = painted_toned(f32::from(pct), Some(Tone::Info));
        let count = assert_clean_bar(&grid, info_fill, track_bg, true, &format!("{pct}%"));
        assert!(count >= prev, "{pct}% shrank from {prev} to {count}");
        prev = count;
    }
    assert_eq!(prev, usize::from(CELLS));

    let accent_fill = painted(100.0).get(0, 0).unwrap().bg();
    for step in 0..=20_u8 {
        let phase = f32::from(step) / 20.0;
        let grid = painted_indeterminate(phase);
        let count = assert_clean_bar(
            &grid,
            accent_fill,
            track_bg,
            false,
            &format!("phase {phase}"),
        );
        assert!(count > 0, "phase {phase} painted no fill");
    }
}
