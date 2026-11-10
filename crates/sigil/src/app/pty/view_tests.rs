#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::style::Token;

use super::rows;

fn screen(bytes: &[u8]) -> vt100::Parser {
    let mut parser = vt100::Parser::new(4, 20, 0);
    parser.process(bytes);
    parser
}

fn color_at(parser: &vt100::Parser, row: usize, byte: usize) -> Option<Token> {
    let rows = rows(parser.screen());
    let row = rows.get(row).unwrap();
    row.runs
        .iter()
        .find(|run| run.start <= byte && byte < run.end)
        .and_then(|run| run.face.color)
}

#[test]
fn red_text_paints_in_the_theme_error_color() {
    let parser = screen(b"\x1b[31mred\x1b[0m plain");
    assert_eq!(color_at(&parser, 0, 0), Some(Token::Error9));
    assert_eq!(color_at(&parser, 0, 5), None);
}

#[test]
fn truecolor_paints_the_exact_color_the_program_asked_for() {
    let parser = screen(b"\x1b[38;2;30;200;60;48;2;1;2;3mok");
    let rows = rows(parser.screen());
    let face = &rows[0].runs[0].face;
    assert_eq!(
        face.ink,
        Some(crate::style::Color {
            r: 30,
            g: 200,
            b: 60,
            a: 255
        })
    );
    assert_eq!(
        face.fill,
        Some(crate::style::Color {
            r: 1,
            g: 2,
            b: 3,
            a: 255
        })
    );
}

#[test]
fn underline_and_dim_reach_the_face() {
    let parser = screen(b"\x1b[4;2mok");
    let rows = rows(parser.screen());
    let face = &rows[0].runs[0].face;
    assert!(face.underline && face.dim);
}

#[test]
fn the_cursor_cell_shows_on_the_accent() {
    let parser = screen(b"ab");
    let rows = rows(parser.screen());
    let cursor = rows[0].runs.iter().find(|run| run.start == 2).unwrap();
    assert_eq!(cursor.face.background, Some(Token::Accent));
}

#[test]
fn a_hidden_cursor_paints_nothing() {
    let parser = screen(b"ab\x1b[?25l");
    let rows = rows(parser.screen());
    assert!(rows[0].runs.iter().all(|run| run.face.background.is_none()));
}

#[test]
fn rows_fill_the_screen_width_so_backgrounds_reach_the_edge() {
    let parser = screen(b"\x1b[44m\x1b[2K");
    let rows = rows(parser.screen());
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].text.chars().count(), 20);
    assert_eq!(
        rows[0].runs.last().unwrap().face.background,
        Some(Token::Info9)
    );
}
