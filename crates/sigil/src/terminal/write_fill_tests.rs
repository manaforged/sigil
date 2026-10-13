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

fn band(top: f32, bottom: f32, ink: Rgba8) -> SceneCommand {
    SceneCommand::Rect(
        DrawRect::new(
            Rect::new(Point::new(0.0, top), 16.0, bottom - top),
            SceneRectStyle::solid(ink),
        )
        .with_cell(CellRect {
            leaf: true,
            ..CellRect::default()
        }),
    )
}

#[test]
fn a_sliver_stacked_on_a_bar_keeps_its_colour_in_partial_blocks() {
    let dark = rgba(6, 4, 11, 255);
    let green = rgba(90, 200, 120, 255);
    let blue = rgba(105, 140, 210, 255);
    let backdrop = SceneCommand::Rect(DrawRect::new(
        Rect::new(Point::new(0.0, 0.0), 16.0, 48.0),
        SceneRectStyle::solid(dark),
    ));
    let grid = draw(
        vec![
            backdrop,
            band(13.0, 16.0, green),
            band(16.0, 20.0, rgba(220, 200, 100, 255)),
            band(20.0, 48.0, blue),
        ],
        2,
        6,
    );
    let cell = |row| grid.get(0, row).expect("plot cell");
    assert_eq!(cell(1).text, "▃");
    assert_eq!(cell(1).fg, Ink::Exact(green));
    assert_eq!(cell(1).bg, Ink::Exact(dark));
    assert_eq!(cell(2).text, "▄");
    assert_eq!(cell(2).fg, Ink::Exact(blue));
    assert_eq!(cell(2).bg, Ink::Exact(rgba(220, 200, 100, 255)));
    assert_eq!((cell(3).text.as_str(), cell(3).bg), (" ", Ink::Exact(blue)));
    assert!(!grid.text().contains('─'), "{}", grid.text());
}

#[test]
fn a_sliver_at_the_floor_stays_inside_its_bounds() {
    let ink = rgba(220, 200, 100, 255);
    let grid = draw(vec![band(30.0, 31.0, ink)], 2, 6);
    assert_eq!(grid.get(0, 3).expect("sliver row").text, "─");
    assert_eq!(grid.get(0, 4).expect("row below").text, " ");
}
