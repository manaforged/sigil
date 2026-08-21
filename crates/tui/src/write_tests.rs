use sigil_hal::{Point, Rect};

use super::{Grid, Ink, paint, paint_for, row_bytes, supports_truecolor};
use crate::paint::raster;
use sigil_app::{Host, Painted};
use sigil_kernel::{Cx, text as text_node};
use sigil_style::{Rgba8, Tone};

/// A `Painted` needs a real `NodeId`, so borrow one from a mounted tree.
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
        bare,
        clip: None,
    }
}

/// The cube is the nearest 256-colour slot, for terminals without 24-bit.
#[test]
fn a_colour_without_truecolor_lands_in_the_cube() {
    use super::cube;
    // Pure red and pure blue are corners of the cube, and distinct.
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
}

#[test]
fn apple_terminal_uses_the_256_colour_cube() {
    assert!(!supports_truecolor(
        Some("Apple_Terminal"),
        Some("truecolor")
    ));
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

/// A resolved colour goes out as 24-bit, not the nearest of eight.
#[test]
fn a_resolved_colour_goes_out_as_truecolor() {
    // The fallback is environment-driven; this asserts the 24-bit path.
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

/// Weight reaches the terminal as an attribute, not a colour.
#[test]
fn bold_text_sets_the_bold_attribute() {
    let mut node = painted("HEAVY", None, false);
    node.weight = 700;
    let grid = raster(&[node], 20, 3, 8.0);
    let out = bytes(&grid, 0);
    assert!(out.contains("\u{1b}[1m"), "expected bold in {out:?}");
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
    // Every way of setting a colour, not one of three: 256-colour,
    // truecolor, and either background form.
    for escape in ["38;5;", "38;2;", "48;5;", "48;2;"] {
        assert!(
            !out.contains(escape),
            "an untoned row must inherit the user's palette, found {escape:?} in {out:?}"
        );
    }
}

/// Ink on its own colour is invisible. A toned text node sets the
/// glyph's colour and leaves the surface behind it alone.
#[test]
fn toned_text_does_not_paint_its_own_background() {
    let grid = raster(&[painted("STRAY", Some(Tone::Warn), false)], 20, 3, 8.0);
    let cell = grid.get(0, 0).expect("first cell");
    assert_eq!(cell.ch, 'S');
    assert_eq!(cell.fg, Ink::Of(Tone::Warn));
    assert_eq!(cell.bg, Ink::Bare, "the glyph must not sit on its own tone");
}

#[test]
fn a_blank_tail_is_cleared_rather_than_spaced() {
    let grid = raster(&[painted("hi", None, false)], 60, 3, 8.0);
    let out = bytes(&grid, 0);
    assert!(
        !out.contains("          "),
        "trailing blanks must not be a run of spaces: {out:?}"
    );
    // The row is short, so the rest of the line must be cleared — without
    // it, last frame's glyphs stay to the right of the new text.
    assert!(
        out.contains("\u{1b}[K"),
        "the tail must be cleared, not just left unwritten: {out:?}"
    );
}
