use super::*;

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
fn explicit_tab_colours_survive_ansi16_tones() {
    use crate::terminal::paint::{CellPaint, Marks, raster_with};

    let scene = CommandScene::new(vec![
        fill(rgba(0, 255, 255, 255), false),
        label(
            "Tasks 2/3",
            rgba(10, 12, 18, 255),
            TextStyle::default(),
            Some(Tone::Info),
        ),
    ]);
    let theme = Theme::dark();
    let marks = Marks::default();
    let mut grid = Grid::new(20, 3);
    raster_with(
        &mut grid,
        &scene,
        &CellPaint {
            theme: &theme,
            cell: 8.0,
            caps: Caps {
                color: ColorDepth::Ansi16,
                ..Caps::default()
            },
            marks: &marks,
        },
    );
    let cell = grid.get(0, 0).expect("tab label");
    assert_eq!(
        paint(cell.fg, ColorDepth::Ansi16),
        crossterm::style::Color::Black
    );
    assert_eq!(
        paint(cell.bg, ColorDepth::Ansi16),
        crossterm::style::Color::Cyan
    );
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
