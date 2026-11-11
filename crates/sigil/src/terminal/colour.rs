use crossterm::style::Color as TermColor;

use crate::style::{Color, Tone};
use crate::terminal::caps::ColorDepth;
use crate::terminal::paint::Ink;

pub(super) const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

pub(crate) fn cube(colour: Color) -> u8 {
    let (red, r) = nearest_level(colour.r);
    let (green, g) = nearest_level(colour.g);
    let (blue, b) = nearest_level(colour.b);
    let cube_distance = distance(colour, red, green, blue);
    let mean = (u16::from(colour.r) + u16::from(colour.g) + u16::from(colour.b)) / 3;
    let gray_index = mean.saturating_sub(8).saturating_add(5) / 10;
    let gray_index = gray_index.min(23) as u8;
    let gray = 8 + gray_index * 10;
    if !chromatic(colour) && distance(colour, gray, gray, gray) < cube_distance {
        232 + gray_index
    } else {
        16 + 36 * r + 6 * g + b
    }
}

fn nearest_level(channel: u8) -> (u8, u8) {
    (0_u8..)
        .zip(CUBE_LEVELS)
        .min_by_key(|(_, level)| channel.abs_diff(*level))
        .map_or((0, 0), |(index, level)| (level, index))
}

fn distance(colour: Color, red: u8, green: u8, blue: u8) -> u32 {
    let square = |value: u8, target: u8| u32::from(value.abs_diff(target)).pow(2);
    square(colour.r, red) + square(colour.g, green) + square(colour.b, blue)
}

pub(crate) fn paint(ink: Ink, depth: ColorDepth) -> TermColor {
    if depth == ColorDepth::Monochrome {
        return TermColor::Reset;
    }
    match ink {
        Ink::Bare | Ink::Of(Tone::Neutral) => TermColor::Reset,
        Ink::Of(Tone::Primary) => TermColor::Blue,
        Ink::Of(Tone::Info) => TermColor::Cyan,
        Ink::Of(Tone::Success) => TermColor::Green,
        Ink::Of(Tone::Warning) => TermColor::Yellow,
        Ink::Of(Tone::Error) => TermColor::Red,
        Ink::Exact(colour) => match depth {
            ColorDepth::Truecolor => TermColor::Rgb {
                r: colour.r,
                g: colour.g,
                b: colour.b,
            },
            ColorDepth::Ansi256 => TermColor::AnsiValue(cube(colour)),
            ColorDepth::Ansi16 | ColorDepth::Monochrome => basic(colour),
        },
    }
}

#[cfg(feature = "scene")]
pub(super) fn rgba(ink: Ink, fallback: Color) -> Color {
    let [r, g, b] = match paint(ink, ColorDepth::Truecolor) {
        TermColor::Reset => return fallback,
        TermColor::Rgb { r, g, b } => [r, g, b],
        color => BASIC
            .iter()
            .find(|(value, _)| *value == color)
            .map_or([fallback.r, fallback.g, fallback.b], |(_, rgb)| *rgb),
    };
    Color { r, g, b, a: 255 }
}

const BASIC: [(TermColor, [u8; 3]); 16] = [
    (TermColor::Black, [0, 0, 0]),
    (TermColor::DarkRed, [205, 0, 0]),
    (TermColor::DarkGreen, [0, 205, 0]),
    (TermColor::DarkYellow, [205, 205, 0]),
    (TermColor::DarkBlue, [0, 0, 238]),
    (TermColor::DarkMagenta, [205, 0, 205]),
    (TermColor::DarkCyan, [0, 205, 205]),
    (TermColor::Grey, [229, 229, 229]),
    (TermColor::DarkGrey, [127, 127, 127]),
    (TermColor::Red, [255, 0, 0]),
    (TermColor::Green, [0, 255, 0]),
    (TermColor::Yellow, [255, 255, 0]),
    (TermColor::Blue, [92, 92, 255]),
    (TermColor::Magenta, [255, 0, 255]),
    (TermColor::Cyan, [0, 255, 255]),
    (TermColor::White, [255, 255, 255]),
];

const CHROMA: u8 = 40;

fn chromatic(colour: Color) -> bool {
    let high = colour.r.max(colour.g).max(colour.b);
    let low = colour.r.min(colour.g).min(colour.b);
    high.saturating_sub(low) >= CHROMA
}

const fn grey(color: TermColor) -> bool {
    matches!(
        color,
        TermColor::Black | TermColor::Grey | TermColor::DarkGrey | TermColor::White
    )
}

fn basic(colour: Color) -> TermColor {
    let hue = chromatic(colour);
    BASIC
        .iter()
        .filter(|(color, _)| !(hue && grey(*color)))
        .min_by_key(|(_, [red, green, blue])| distance(colour, *red, *green, *blue))
        .map_or(TermColor::Reset, |(color, _)| *color)
}

#[cfg(test)]
#[path = "colour_tests.rs"]
mod tests;
