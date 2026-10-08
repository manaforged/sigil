use super::*;

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
    let block = label("█", blue, TextStyle::default(), None);
    let grid = draw(vec![fill(dark, false), block], 2, 1);
    let cell = grid.get(0, 0).expect("full-block cell");
    assert_ne!(cell.fg, cell.bg, "the grid must preserve authored colours");
    assert_eq!(terminal_inks(&cell), (cell.fg, cell.fg));
}

#[test]
fn a_leaf_fill_covers_the_cell_without_a_block_glyph() {
    let ink = Rgba8 {
        r: 148,
        g: 156,
        b: 176,
        a: 255,
    };
    let grid = draw(vec![fill(ink, true)], 2, 1);
    let cell = grid.get(0, 0).expect("filled cell");
    assert_eq!(cell.text, " ");
    assert_eq!(cell.bg, Ink::Exact(ink));
}

#[test]
fn a_half_cell_bar_is_solid_and_a_one_pixel_rule_stays_thin() {
    let ink = Rgba8 {
        r: 148,
        g: 156,
        b: 176,
        a: 255,
    };
    let paint = |width: f32| {
        let command = SceneCommand::Rect(
            DrawRect::new(
                Rect::new(Point::new(0.0, 0.0), width, 48.0),
                SceneRectStyle::solid(ink),
            )
            .with_cell(CellRect {
                leaf: true,
                ..CellRect::default()
            }),
        );
        raster(
            &CommandScene::new(vec![command]),
            &Theme::default(),
            2,
            6,
            8.0,
        )
    };
    let bar = paint(4.0);
    let cell = bar.get(0, 2).expect("bar cell");
    assert_eq!(cell.text, " ");
    assert_eq!(cell.bg, Ink::Exact(ink));
    let rule = paint(1.0);
    assert_eq!(rule.get(0, 2).expect("rule cell").text, "│");
}

#[test]
fn a_fill_over_a_bare_block_drops_the_block_glyph() {
    let scrim = Rgba8 {
        r: 0,
        g: 0,
        b: 0,
        a: 153,
    };
    let field = Rgba8 {
        r: 33,
        g: 34,
        b: 37,
        a: 255,
    };
    let grid = draw(vec![fill(scrim, true), fill(field, false)], 4, 1);
    let cell = grid.get(0, 0).expect("covered cell");
    assert_eq!(cell.text, " ");
    assert_eq!(cell.bg, Ink::Exact(field));
}
