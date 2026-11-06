use crate::style::{Rgba8, TextFace, TextRun, Token};

use super::Row;

const BASE: [Token; 16] = [
    Token::Gray3,
    Token::Error9,
    Token::Success9,
    Token::Warn9,
    Token::Info9,
    Token::Accent9,
    Token::Info11,
    Token::Gray11,
    Token::Gray8,
    Token::Error11,
    Token::Success11,
    Token::Warn11,
    Token::Info11,
    Token::Accent11,
    Token::Info12,
    Token::Gray12,
];

pub(super) fn rows(screen: &vt100::Screen) -> Vec<Row> {
    let (height, width) = screen.size();
    let cursor = (!screen.hide_cursor() && screen.scrollback() == 0).then(|| screen.cursor_position());
    (0..height)
        .map(|row| line(screen, row, width, cursor.filter(|at| at.0 == row).map(|at| at.1)))
        .collect()
}

fn line(screen: &vt100::Screen, row: u16, width: u16, cursor: Option<u16>) -> Row {
    let mut out = Row::default();
    for col in 0..width {
        let Some(cell) = screen.cell(row, col) else {
            continue;
        };
        if cell.is_wide_continuation() {
            continue;
        }
        let start = out.text.len();
        let contents = cell.contents();
        out.text.push_str(if contents.is_empty() { " " } else { contents });
        let mut face = face(cell);
        if cursor == Some(col) {
            face.background = Some(Token::Accent);
            face.color = Some(Token::OnAccent);
        }
        push(&mut out.runs, start, out.text.len(), face);
    }
    out
}

fn push(runs: &mut Vec<TextRun>, start: usize, end: usize, face: TextFace) {
    if let Some(last) = runs.last_mut()
        && last.end == start
        && last.face == face
    {
        last.end = end;
        return;
    }
    runs.push(TextRun { start, end, face });
}

fn face(cell: &vt100::Cell) -> TextFace {
    let (mut front, mut back) = (ink(cell.fgcolor()), ink(cell.bgcolor()));
    if cell.inverse() {
        (front, back) = (
            back.or(Some(Ink::Token(Token::Surface))),
            front.or(Some(Ink::Token(Token::OnSurface))),
        );
    }
    let (color, ink) = split(front);
    let (background, fill) = split(back);
    TextFace {
        bold: cell.bold(),
        italic: cell.italic(),
        underline: cell.underline(),
        dim: cell.dim(),
        mono: true,
        square: true,
        color,
        background,
        ink,
        fill,
        ..TextFace::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ink {
    Token(Token),
    Exact(Rgba8),
}

const fn split(ink: Option<Ink>) -> (Option<Token>, Option<Rgba8>) {
    match ink {
        None => (None, None),
        Some(Ink::Token(token)) => (Some(token), None),
        Some(Ink::Exact(color)) => (None, Some(color)),
    }
}

const fn exact(r: u8, g: u8, b: u8) -> Ink {
    Ink::Exact(Rgba8 { r, g, b, a: 255 })
}

fn ink(color: vt100::Color) -> Option<Ink> {
    match color {
        vt100::Color::Default => None,
        vt100::Color::Idx(index) => Some(indexed(index)),
        vt100::Color::Rgb(red, green, blue) => Some(exact(red, green, blue)),
    }
}

fn indexed(index: u8) -> Ink {
    match index {
        0..=15 => Ink::Token(BASE[usize::from(index)]),
        16..=231 => {
            let level = |step: u8| if step == 0 { 0 } else { 55 + step * 40 };
            let cube = index - 16;
            exact(level(cube / 36), level(cube / 6 % 6), level(cube % 6))
        }
        _ => {
            let gray = 8 + (index - 232) * 10;
            exact(gray, gray, gray)
        }
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
