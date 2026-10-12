use super::*;

#[test]
fn a_full_block_fills_font_seams_with_its_ink() {
    let dark = rgba(6, 4, 11, 255);
    let blue = rgba(105, 140, 210, 255);
    let block = label("█", blue, TextStyle::default(), None);
    let grid = draw(vec![fill(dark, false), block], 2, 1);
    let cell = grid.get(0, 0).expect("full-block cell");
    assert_ne!(cell.fg, cell.bg, "the grid must preserve authored colours");
    assert_eq!(terminal_inks(&cell), (cell.fg, cell.fg));
}

#[test]
fn a_leaf_fill_covers_the_cell_without_a_block_glyph() {
    let ink = rgba(148, 156, 176, 255);
    let grid = draw(vec![fill(ink, true)], 2, 1);
    let cell = grid.get(0, 0).expect("filled cell");
    assert_eq!(cell.text, " ");
    assert_eq!(cell.bg, Ink::Exact(ink));
}

#[test]
fn a_half_cell_bar_is_solid_and_a_one_pixel_rule_stays_thin() {
    let ink = rgba(148, 156, 176, 255);
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
    let scrim = rgba(0, 0, 0, 153);
    let field = rgba(33, 34, 37, 255);
    let grid = draw(vec![fill(scrim, true), fill(field, false)], 4, 1);
    let cell = grid.get(0, 0).expect("covered cell");
    assert_eq!(cell.text, " ");
    assert_eq!(cell.bg, Ink::Exact(field));
}

#[test]
fn control_characters_in_text_never_reach_the_terminal() {
    let ink = rgba(230, 230, 230, 255);
    let text = "a\x1b]52;c;eA==\x07b\tc\nd\x1b[2Je";
    let grid = draw(vec![label(text, ink, TextStyle::default(), None)], 40, 1);
    let shown: String = (0..40)
        .filter_map(|x| grid.get(x, 0))
        .map(|cell| cell.text.to_string())
        .collect();
    assert!(!shown.chars().any(char::is_control), "{shown:?}");
    assert!(shown.starts_with('a'), "{shown:?}");
}
