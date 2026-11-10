#![allow(clippy::shadow_unrelated)]

use crate::geometry::{Point, Rect};

use super::{AnsiPen, Cell, Grid, Ink, paint, row_bytes, terminal_inks};
use crate::app::{CellWidth, set_cell_width};
use crate::scene::command::CellRect;
use crate::scene::{CommandScene, DrawRect, GlyphRun, SceneCommand, TextFlow};
use crate::style::{Rgba8, TextStyle, Theme, Tone};
use crate::terminal::caps::{Caps, ColorDepth};
use crate::terminal::paint::raster;

const UNSET: Rgba8 = rgba(0, 0, 0, 0);

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

fn area() -> Rect {
    Rect::new(Point::new(0.0, 0.0), 8.0 * 8.0, 8.0)
}

fn label(text: &str, ink: Rgba8, style: TextStyle, tone: Option<Tone>) -> SceneCommand {
    let run = GlyphRun::new(area(), text, ink, style, 10_000).with_cell(TextFlow::Wrap, tone);
    SceneCommand::GlyphRun(run)
}

fn plain(text: &str, tone: Option<Tone>) -> SceneCommand {
    label(text, UNSET, TextStyle::default(), tone)
}

fn fill(color: Rgba8, leaf: bool) -> SceneCommand {
    SceneCommand::Rect(DrawRect::new(area(), color).with_cell(CellRect {
        leaf,
        ..CellRect::default()
    }))
}

fn draw(commands: Vec<SceneCommand>, cols: u16, rows: u16) -> Grid {
    raster(
        &CommandScene::new(commands),
        &Theme::default(),
        cols,
        rows,
        8.0,
    )
}

#[test]
fn terminal_output_keeps_graphemes_in_their_display_columns() {
    for (mode, widths) in [
        (CellWidth::Unicode, [2, 1, 2, 2, 2, 1]),
        (
            CellWidth::Scalar {
                joiner: 1,
                nonjoiner: 1,
                keycap: 1,
            },
            [4, 1, 5, 2, 2, 2],
        ),
    ] {
        set_cell_width(mode);
        for (symbol, width) in ["👍🏽", "e\u{301}", "👩\u{200d}💻", "🇳🇿", "1️⃣", "a\u{200c}"]
            .into_iter()
            .zip(widths)
        {
            let text = format!("A{symbol}B│");
            let grid = draw(vec![plain(&text, None)], 8, 1);
            assert_eq!(grid.text(), text);
            assert_eq!(grid.painted_width(0), width + 3);
            assert!(bytes(&grid, 0).contains(&text));
        }
    }
    set_cell_width(CellWidth::Unicode);
}

fn bytes(grid: &Grid, row: usize) -> String {
    let mut out: Vec<u8> = Vec::new();
    row_bytes(
        &mut out,
        grid,
        row,
        &mut AnsiPen::default(),
        &Caps::default(),
    )
    .expect("write");
    String::from_utf8(out).expect("utf8")
}

#[test]
fn a_blank_tail_is_cleared_rather_than_spaced() {
    let grid = draw(vec![plain("hi", None)], 60, 3);
    let out = bytes(&grid, 0);
    assert!(
        !out.contains("          "),
        "trailing blanks must not be a run of spaces: {out:?}"
    );
    assert!(
        out.contains("\u{1b}[K"),
        "the tail must be cleared, not just left unwritten: {out:?}"
    );
}

#[test]
fn inline_proof_paints_relative_and_clears_tail() {
    let caps = Caps::default();
    let grid6 = draw(vec![plain("alpha", None)], 8, 6);

    let mut out = Vec::new();
    super::inline(&mut out, None, &grid6, (0, false), &caps).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(!text.contains("\x1b[1049h"), "inline emitted alt-screen");
    assert!(
        !text.contains("\x1b[H") && !text.contains("H\x1b"),
        "absolute MoveTo"
    );
    for row in 1..=20u8 {
        let needle = format!("\x1b[{row};");
        assert!(!text.contains(&needle), "absolute row {row}");
    }
    assert!(text.contains("alpha"));
    assert_eq!(text.matches("\r\n").count(), 5);

    let grid4 = draw(vec![plain("beta", None)], 8, 4);
    let mut shrunk_out = Vec::new();
    super::inline(&mut shrunk_out, None, &grid4, (6, false), &caps).unwrap();
    let shrunk = String::from_utf8(shrunk_out).unwrap();
    assert!(
        shrunk.contains("\x1b[5A"),
        "shrink must return to region top: {shrunk:?}"
    );
    assert_eq!(
        shrunk.matches("\x1b[2K").count(),
        6,
        "tail rows need clearing"
    );
    assert_eq!(shrunk.matches("\r\n").count(), 5);
}

#[test]
fn the_cursor_shows_only_at_a_text_caret() -> std::io::Result<()> {
    let caps = Caps::default();
    let mut focus = Vec::new();
    super::cursor(&mut focus, Some((0, 0)), false, &caps)?;
    let mut caret = Vec::new();
    super::cursor(&mut caret, Some((3, 1)), true, &caps)?;
    let focus = String::from_utf8_lossy(&focus);
    let caret = String::from_utf8_lossy(&caret);
    assert!(
        focus.contains("\x1b[?25l") && !focus.contains("\x1b[?25h"),
        "a focus spot without a caret must keep the cursor hidden: {focus:?}"
    );
    assert!(
        caret.contains("\x1b[2;4H") && caret.ends_with("\x1b[?25h"),
        "a text caret shows the cursor where it sits: {caret:?}"
    );
    Ok(())
}

#[test]
fn a_painted_last_cell_keeps_its_background() {
    let scrim = rgba(10, 8, 20, 255);
    let star = rgba(176, 200, 240, 255);
    let cells: Vec<Cell> = (0..8)
        .map(|column| {
            let (text, color) = if column == 5 {
                ("·", star)
            } else {
                (" ", scrim)
            };
            Cell {
                text: text.into(),
                fg: Ink::Exact(color),
                bg: Ink::Exact(color),
                ..Cell::blank()
            }
        })
        .collect();
    let mut grid = Grid::new(8, 1);
    grid.set_row(0, &cells);
    let out = bytes(&grid, 0);
    assert!(
        out.contains('·') && !out.contains("\u{1b}[K"),
        "the last cell was erased to the terminal background: {out:?}"
    );
}

#[path = "write_colour_tests.rs"]
mod colours;

#[path = "write_fill_tests.rs"]
mod fills;

#[path = "write_link_tests.rs"]
mod links;

#[test]
fn a_row_with_changes_far_apart_rewrites_only_the_changed_cells() {
    let caps = Caps::default();
    let row: Vec<Cell> = (0..40)
        .map(|index| Cell {
            text: if index % 2 == 0 { "x" } else { "y" }.into(),
            ..Cell::blank()
        })
        .collect();
    let mut prev = Grid::new(40, 1);
    prev.set_row(0, &row);
    let changed: Vec<Cell> = row
        .iter()
        .enumerate()
        .map(|(index, cell)| match index {
            2 => Cell {
                text: "A".into(),
                ..cell.clone()
            },
            37 => Cell {
                text: "B".into(),
                ..cell.clone()
            },
            _ => cell.clone(),
        })
        .collect();
    let mut next = Grid::new(40, 1);
    next.set_row(0, &changed);
    let mut out = Vec::new();
    super::dirty(&mut out, &prev, &next, &caps, None).expect("write");
    let text = String::from_utf8(out).expect("utf8");
    assert!(text.contains('A') && text.contains('B'), "{text:?}");
    assert!(
        !text.contains("xyxyxy"),
        "rewrote the unchanged middle: {text:?}"
    );
}

#[test]
fn a_row_limited_write_matches_the_full_write() {
    let caps = Caps::default();
    let base: Vec<Cell> = (0..20)
        .map(|index| Cell {
            text: if index % 2 == 0 {
                "x".into()
            } else {
                "y".into()
            },
            ..Cell::default()
        })
        .collect();
    let mut prev = Grid::new(20, 64);
    for row in 0..64 {
        prev.set_row(row, &base);
    }
    let mut next = prev.clone();
    let mut changed = base.clone();
    if let Some(cell) = changed.get_mut(7) {
        *cell = Cell {
            text: "Z".into(),
            ..Cell::default()
        };
    }
    next.set_row(41, &changed);
    let mut full = Vec::new();
    super::dirty(&mut full, &prev, &next, &caps, None).expect("write");
    let mut limited = Vec::new();
    let damage = super::diff::RowDamage::new(vec![41], false);
    super::dirty(&mut limited, &prev, &next, &caps, Some(&damage)).expect("write");
    assert_eq!(
        String::from_utf8_lossy(&limited),
        String::from_utf8_lossy(&full)
    );
}
