use sigil_hal::{Point, Rect};

use super::{Grid, Ink, paint, paint_for, row_bytes, supports_truecolor, terminal_inks};
use crate::paint::raster;
use sigil_app::{Host, Painted};
use sigil_kernel::{Cx, text as text_node};
use sigil_style::{Rgba8, Tone};

fn painted(label: &str, tone: Option<Tone>, bare: bool) -> Painted {
    let mut host = Host::cells();
    host.mount(|cx: &mut Cx| cx.add(text_node("seed")));
    host.layout((80.0, 24.0)).expect("layout");
    let id = host.painted().first().expect("one node").id;
    Painted {
        id,
        rect: Rect::new(Point::new(0.0, 0.0), 8.0 * 8.0, 8.0),
        text: (!bare).then(|| label.to_owned()),
        tone,
        fill: None,
        ink: None,
        weight: 400,
        italic: false,
        bare,
        clip: None,
    }
}

#[test]
fn a_colour_without_truecolor_lands_in_the_cube() {
    use super::cube;
    assert_eq!(
        cube(Rgba8 {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        }),
        196
    );
    assert_eq!(
        cube(Rgba8 {
            r: 0,
            g: 0,
            b: 255,
            a: 255
        }),
        21
    );
    assert_eq!(
        cube(Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 255
        }),
        16
    );
    assert_eq!(
        cube(Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255
        }),
        231
    );
    assert_eq!(
        cube(Rgba8 {
            r: 24,
            g: 15,
            b: 46,
            a: 255,
        }),
        234,
        "a dark violet is closer to xterm gray 28 than saturated blue"
    );
}

#[test]
fn apple_terminal_honors_an_explicit_truecolor_capability() {
    assert!(supports_truecolor(
        Some("Apple_Terminal"),
        Some("truecolor")
    ));
    assert!(!supports_truecolor(Some("Apple_Terminal"), None));
    assert!(supports_truecolor(Some("WezTerm"), Some("truecolor")));
    assert_eq!(
        paint_for(
            Ink::Exact(Rgba8 {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            }),
            false,
        ),
        crossterm::style::Color::AnsiValue(196)
    );
}

#[test]
fn a_resolved_colour_goes_out_as_truecolor() {
    if !super::truecolor() {
        return;
    }
    let exact = Rgba8 {
        r: 210,
        g: 40,
        b: 90,
        a: 255,
    };
    assert_eq!(
        paint(Ink::Exact(exact)),
        crossterm::style::Color::Rgb {
            r: 210,
            g: 40,
            b: 90,
        }
    );
    if crossterm::style::Colored::ansi_color_disabled_memoized() {
        return;
    }
    let mut node = painted("EXACT", None, false);
    node.ink = Some(exact);
    let grid = raster(&[node], 20, 3, 8.0);
    let out = bytes(&grid, 0);
    assert!(
        out.contains("38;2;210;40;90"),
        "expected a 24-bit foreground in {out:?}"
    );
}

#[test]
fn bold_text_sets_the_bold_attribute() {
    let mut node = painted("HEAVY", None, false);
    node.weight = 700;
    let grid = raster(&[node], 20, 3, 8.0);
    let out = bytes(&grid, 0);
    assert!(out.contains("\u{1b}[1m"), "expected bold in {out:?}");
}

#[test]
fn italic_text_sets_the_italic_attribute() {
    let mut node = painted("THOUGHT", None, false);
    node.italic = true;
    let grid = raster(&[node], 20, 3, 8.0);
    assert!(grid.get(0, 0).is_some_and(|cell| cell.italic));
    let out = bytes(&grid, 0);
    assert!(out.contains("\u{1b}[3m"), "expected italic in {out:?}");
}

fn bytes(grid: &Grid, row: usize) -> String {
    let mut out: Vec<u8> = Vec::new();
    let mut line = String::new();
    row_bytes(&mut out, grid, row, &mut line).expect("write");
    String::from_utf8(out).expect("utf8")
}

#[test]
fn a_toned_glyph_uses_the_warning_colour() {
    let grid = raster(&[painted("STRAY", Some(Tone::Warn), false)], 20, 3, 8.0);
    let out = bytes(&grid, 0);
    assert!(out.contains("STRAY"), "expected the text in {out:?}");
    assert_eq!(paint(Ink::Of(Tone::Warn)), crossterm::style::Color::Yellow);
    if crossterm::style::Colored::ansi_color_disabled_memoized() {
        assert!(
            !out.contains("38;5;11m"),
            "NO_COLOR must suppress the yellow SGR in {out:?}"
        );
    } else {
        assert!(out.contains("5;11m"), "expected a yellow SGR in {out:?}");
    }
    assert!(out.contains("\u{1b}[0m"), "expected a reset in {out:?}");
}

#[test]
fn an_untoned_row_sets_no_colour() {
    let grid = raster(&[painted("plain", None, false)], 20, 3, 8.0);
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
fn toned_text_does_not_paint_its_own_background() {
    let grid = raster(&[painted("STRAY", Some(Tone::Warn), false)], 20, 3, 8.0);
    let cell = grid.get(0, 0).expect("first cell");
    assert_eq!(cell.ch, 'S');
    assert_eq!(cell.fg, Ink::Of(Tone::Warn));
    assert_eq!(cell.bg, Ink::Bare, "the glyph must not sit on its own tone");
}

#[test]
fn a_full_block_fills_font_seams_with_its_ink() {
    let dark = Rgba8 {
        r: 6,
        g: 4,
        b: 11,
        a: 255,
    };
    let blue = Rgba8 {
        r: 105,
        g: 140,
        b: 210,
        a: 255,
    };
    let mut background = painted("", None, false);
    background.text = None;
    background.fill = Some(dark);
    let mut block = painted("█", None, false);
    block.ink = Some(blue);
    let grid = raster(&[background, block], 2, 1, 8.0);
    let cell = grid.get(0, 0).expect("full-block cell");
    assert_ne!(cell.fg, cell.bg, "the grid must preserve authored colours");
    assert_eq!(terminal_inks(cell), (cell.fg, cell.fg));
}

#[test]
fn a_blank_tail_is_cleared_rather_than_spaced() {
    let grid = raster(&[painted("hi", None, false)], 60, 3, 8.0);
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
