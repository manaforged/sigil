use std::borrow::Cow;

use crate::terminal::caps::Caps;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Glyphs {
    #[default]
    Unicode,
    Console,
    Ascii,
}

const BRAILLE: std::ops::RangeInclusive<char> = '\u{2800}'..='\u{28ff}';

#[must_use]
pub fn glyphs() -> Glyphs {
    Caps::detect(|key: &str| std::env::var(key).ok()).glyphs
}

impl Glyphs {
    #[must_use]
    pub fn glyph(self, glyph: char) -> char {
        match self {
            Self::Unicode => glyph,
            Self::Console => console(glyph).unwrap_or(glyph),
            Self::Ascii => ascii(glyph).unwrap_or(glyph),
        }
    }

    #[must_use]
    pub fn text(self, text: &str) -> Cow<'_, str> {
        if self == Self::Unicode || text.is_ascii() {
            return Cow::Borrowed(text);
        }
        Cow::Owned(text.chars().map(|glyph| self.glyph(glyph)).collect())
    }
}

fn console(glyph: char) -> Option<char> {
    let kept = matches!(
        glyph,
        '█' | '▀'
            | '▄'
            | '░'
            | '▒'
            | '▓'
            | '─'
            | '│'
            | '┌'
            | '┐'
            | '└'
            | '┘'
            | '├'
            | '┤'
            | '┬'
            | '┴'
            | '┼'
            | '↑'
            | '↓'
            | '←'
            | '→'
            | '▲'
            | '▼'
            | '·'
            | '°'
    );
    if kept {
        return Some(glyph);
    }
    let near = match glyph {
        '╭' | '┏' => '┌',
        '╮' | '┓' => '┐',
        '╰' | '┗' => '└',
        '╯' | '┛' => '┘',
        '━' => '─',
        '┃' => '│',
        '▌' | '▋' | '▊' | '▉' | '▆' | '▇' => '█',
        '▃' | '▅' => '▄',
        _ => return ascii(glyph),
    };
    Some(near)
}

fn ascii(glyph: char) -> Option<char> {
    blocks(glyph)
        .or_else(|| lines(glyph))
        .or_else(|| marks(glyph))
        .or_else(|| BRAILLE.contains(&glyph).then(|| braille(glyph)))
}

const fn blocks(glyph: char) -> Option<char> {
    Some(match glyph {
        '▏' | '▎' | '▍' => '|',
        '▌' | '▋' | '▊' | '▉' | '█' | '▇' | '▓' => '#',
        '▁' => '_',
        '▂' | '░' => '.',
        '▃' => '-',
        '▄' => '=',
        '▅' => '+',
        '▆' => '*',
        '▀' => '"',
        '▒' => ':',
        _ => return None,
    })
}

const fn lines(glyph: char) -> Option<char> {
    Some(match glyph {
        '─' | '━' => '-',
        '│' | '┃' => '|',
        '┌' | '┐' | '└' | '┘' | '├' | '┤' | '┬' | '┴' | '┼' | '╭' | '╮' | '╰' | '╯' | '┏' | '┓'
        | '┗' | '┛' => '+',
        _ => return None,
    })
}

const fn marks(glyph: char) -> Option<char> {
    Some(match glyph {
        '↑' | '▲' => '^',
        '↓' | '▼' => 'v',
        '←' | '◀' => '<',
        '→' | '▶' => '>',
        '·' | '…' => '.',
        '°' | '○' => 'o',
        '●' | '•' => '*',
        _ => return None,
    })
}

fn braille(glyph: char) -> char {
    match u32::from(glyph).saturating_sub(0x2800).count_ones() {
        0 => ' ',
        1 | 2 => '.',
        3 | 4 => ':',
        _ => '#',
    }
}

#[cfg(test)]
#[path = "glyphs_tests.rs"]
mod tests;
