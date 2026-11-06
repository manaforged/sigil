use crate::style::{TextFace, TextRun, Token};

use super::Row;

const GRAYS: [Token; 12] = [
    Token::Gray1,
    Token::Gray2,
    Token::Gray3,
    Token::Gray4,
    Token::Gray5,
    Token::Gray6,
    Token::Gray7,
    Token::Gray8,
    Token::Gray9,
    Token::Gray10,
    Token::Gray11,
    Token::Gray12,
];

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
    let (mut color, mut background) = (token(cell.fgcolor()), token(cell.bgcolor()));
    if cell.inverse() {
        (color, background) = (
            background.or(Some(Token::Surface)),
            color.or(Some(Token::OnSurface)),
        );
    }
    TextFace {
        bold: cell.bold(),
        italic: cell.italic(),
        mono: true,
        color,
        background,
        ..TextFace::default()
    }
}

pub(super) fn token(color: vt100::Color) -> Option<Token> {
    match color {
        vt100::Color::Default => None,
        vt100::Color::Idx(index) => Some(indexed(index)),
        vt100::Color::Rgb(red, green, blue) => Some(nearest(red, green, blue)),
    }
}

fn indexed(index: u8) -> Token {
    match index {
        0..=15 => BASE[usize::from(index)],
        16..=231 => {
            let level = |step: u8| if step == 0 { 0 } else { 55 + step * 40 };
            let cube = index - 16;
            nearest(level(cube / 36), level(cube / 6 % 6), level(cube % 6))
        }
        _ => {
            let gray = 8 + (index - 232) * 10;
            nearest(gray, gray, gray)
        }
    }
}

fn nearest(red: u8, green: u8, blue: u8) -> Token {
    let (red, green, blue) = (f32::from(red), f32::from(green), f32::from(blue));
    let high = red.max(green).max(blue);
    let low = red.min(green).min(blue);
    let light = (high + low) / 510.0;
    if high - low < 40.0 {
        return GRAYS[(light * 11.0).round() as usize];
    }
    let hue = if (high - red).abs() < f32::EPSILON {
        60.0 * ((green - blue) / (high - low)).rem_euclid(6.0)
    } else if (high - green).abs() < f32::EPSILON {
        60.0 * ((blue - red) / (high - low) + 2.0)
    } else {
        60.0 * ((red - green) / (high - low) + 4.0)
    };
    let bright = light > 0.6;
    let pick = |dim: Token, lit: Token| if bright { lit } else { dim };
    match hue as u16 {
        0..=19 | 330..=360 => pick(Token::Error9, Token::Error11),
        20..=69 => pick(Token::Warn9, Token::Warn11),
        70..=169 => pick(Token::Success9, Token::Success11),
        170..=259 => pick(Token::Info9, Token::Info11),
        _ => pick(Token::Accent9, Token::Accent11),
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
