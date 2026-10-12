#![allow(clippy::shadow_unrelated)]

use crate::geometry::{Point, Rect};

use super::{Cell, Grid, Ink, Pen, paint, row_bytes, terminal_inks};
use crate::app::{CellWidth, set_cell_width};
use crate::scene::{
    CellRect, CommandScene, DrawGlyphRun, DrawRect, GlyphRun, SceneCommand, SceneRectStyle,
    TextFlow,
};
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
    SceneCommand::GlyphRun(DrawGlyphRun::new(run))
}

fn plain(text: &str, tone: Option<Tone>) -> SceneCommand {
    label(text, UNSET, TextStyle::default(), tone)
}

fn fill(color: Rgba8, leaf: bool) -> SceneCommand {
    SceneCommand::Rect(
        DrawRect::new(area(), SceneRectStyle::solid(color)).with_cell(CellRect {
            leaf,
            ..CellRect::default()
        }),
    )
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

#[test]
fn a_colour_without_truecolor_lands_in_the_cube() {
    use crate::terminal::colour::cube;
    assert_eq!(cube(rgba(255, 0, 0, 255)), 196);
    assert_eq!(cube(rgba(0, 0, 255, 255)), 21);
    assert_eq!(cube(rgba(0, 0, 0, 255)), 16);
    assert_eq!(cube(rgba(255, 255, 255, 255)), 231);
    assert_eq!(
        cube(rgba(24, 15, 46, 255)),
        234,
        "a dark violet is closer to xterm gray 28 than saturated blue"
    );
}

#[test]
fn apple_terminal_honors_an_explicit_truecolor_capability() {
    let depth = |program: &str, colorterm: Option<&str>| {
        Caps::detect(|key| match key {
            "TERM_PROGRAM" => Some(program.to_owned()),
            "COLORTERM" => colorterm.map(str::to_owned),
            _ => None,
        })
        .color
    };
    assert_eq!(
        depth("Apple_Terminal", Some("truecolor")),
        ColorDepth::Truecolor
    );
    assert_ne!(depth("Apple_Terminal", None), ColorDepth::Truecolor);
    assert_eq!(depth("WezTerm", Some("truecolor")), ColorDepth::Truecolor);
    assert_eq!(
        paint(Ink::Exact(rgba(255, 0, 0, 255)), ColorDepth::Ansi256,),
        crossterm::style::Color::AnsiValue(196)
    );
}

#[test]
fn a_resolved_colour_goes_out_as_truecolor() {
    let exact = rgba(210, 40, 90, 255);
    assert_eq!(
        paint(Ink::Exact(exact), ColorDepth::Truecolor),
        crossterm::style::Color::Rgb {
            r: 210,
            g: 40,
            b: 90,
        }
    );
    if crossterm::style::Colored::ansi_color_disabled_memoized() {
        return;
    }
    let grid = draw(
        vec![label("EXACT", exact, TextStyle::default(), None)],
        20,
        3,
    );
    let out = bytes(&grid, 0);
    assert!(
        out.contains("38;2;210;40;90"),
        "expected a 24-bit foreground in {out:?}"
    );
}

#[test]
fn bold_and_italic_text_set_their_attributes() {
    let heavy = TextStyle {
        font_weight: 700,
        ..TextStyle::default()
    };
    let out = bytes(&draw(vec![label("HEAVY", UNSET, heavy, None)], 20, 3), 0);
    assert!(out.contains("\u{1b}[1m"), "expected bold in {out:?}");
    let slanted = TextStyle {
        italic: true,
        ..TextStyle::default()
    };
    let grid = draw(vec![label("THOUGHT", UNSET, slanted, None)], 20, 3);
    assert!(grid.get(0, 0).is_some_and(|cell| cell.italic));
    let out = bytes(&grid, 0);
    assert!(out.contains("\u{1b}[3m"), "expected italic in {out:?}");
}

fn bytes(grid: &Grid, row: usize) -> String {
    let mut out: Vec<u8> = Vec::new();
    row_bytes(&mut out, grid, row, &mut Pen::default(), &Caps::default()).expect("write");
    String::from_utf8(out).expect("utf8")
}

#[test]
fn a_toned_glyph_uses_the_warning_colour() {
    let grid = draw(vec![plain("STRAY", Some(Tone::Warn))], 20, 3);
    let out = bytes(&grid, 0);
    assert!(out.contains("STRAY"), "expected the text in {out:?}");
    assert_eq!(
        paint(Ink::Of(Tone::Warn), ColorDepth::Truecolor),
        crossterm::style::Color::Yellow
    );
    if crossterm::style::Colored::ansi_color_disabled_memoized() {
        assert!(
            !out.contains("38;5;11m"),
            "NO_COLOR must suppress the yellow SGR in {out:?}"
        );
    } else {
        assert!(out.contains("5;11m"), "expected a yellow SGR in {out:?}");
    }
    assert!(out.contains("\u{1b}[0m"), "expected a reset in {out:?}");
    let cell = grid.get(0, 0).expect("first cell");
    assert_eq!(cell.text, "S");
    assert_eq!(cell.fg, Ink::Of(Tone::Warn));
    assert_eq!(cell.bg, Ink::Bare, "the glyph must not sit on its own tone");
}

#[test]
fn an_untoned_row_sets_no_colour() {
    let grid = draw(vec![plain("plain", None)], 20, 3);
    let out = bytes(&grid, 0);
    assert!(out.contains("plain"), "expected the text in {out:?}");
    for escape in ["38;5;", "38;2;", "48;5;", "48;2;"] {
        assert!(
            !out.contains(escape),
            "an untoned row must inherit the user's palette, found {escape:?} in {out:?}"
        );
    }
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

#[path = "write_fill_tests.rs"]
mod fills;

#[path = "write_link_tests.rs"]
mod links;
