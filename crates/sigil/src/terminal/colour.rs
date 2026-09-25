use crossterm::style::Color;

use crate::style::{Rgba8, Tone};
use crate::terminal::caps::ColorDepth;
use crate::terminal::paint::Ink;

pub(crate) fn cube(colour: Rgba8) -> u8 {
    let (red, r) = nearest_level(colour.r);
    let (green, g) = nearest_level(colour.g);
    let (blue, b) = nearest_level(colour.b);
    let cube_distance = distance(colour, red, green, blue);
    let mean = (u16::from(colour.r) + u16::from(colour.g) + u16::from(colour.b)) / 3;
    let gray_index = mean.saturating_sub(8).saturating_add(5) / 10;
    let gray_index = gray_index.min(23) as u8;
    let gray = 8 + gray_index * 10;
    if distance(colour, gray, gray, gray) < cube_distance {
        232 + gray_index
    } else {
        16 + 36 * r + 6 * g + b
    }
}

fn nearest_level(channel: u8) -> (u8, u8) {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    let mut best = (LEVELS[0], 0_u8);
    let mut delta = channel.abs_diff(LEVELS[0]);
    for (index, level) in (1_u8..).zip(LEVELS.into_iter().skip(1)) {
        if channel.abs_diff(level) < delta {
            best = (level, index);
            delta = channel.abs_diff(level);
        }
    }
    best
}

fn distance(colour: Rgba8, red: u8, green: u8, blue: u8) -> u32 {
    let square = |value: u8, target: u8| u32::from(value.abs_diff(target)).pow(2);
    square(colour.r, red) + square(colour.g, green) + square(colour.b, blue)
}

pub(crate) fn paint(ink: Ink, depth: ColorDepth) -> Color {
    if depth == ColorDepth::Monochrome {
        return Color::Reset;
    }
    match ink {
        Ink::Bare | Ink::Of(Tone::Neutral) => Color::Reset,
        Ink::Of(Tone::Primary) => Color::Blue,
        Ink::Of(Tone::Info) => Color::Cyan,
        Ink::Of(Tone::Success) => Color::Green,
        Ink::Of(Tone::Warn) => Color::Yellow,
        Ink::Of(Tone::Error) => Color::Red,
        Ink::Exact(colour) => match depth {
            ColorDepth::Truecolor => Color::Rgb {
                r: colour.r,
                g: colour.g,
                b: colour.b,
            },
            ColorDepth::Ansi256 => Color::AnsiValue(cube(colour)),
            ColorDepth::Ansi16 | ColorDepth::Monochrome => basic(colour),
        },
    }
}

const BASIC: [(Color, [u8; 3]); 16] = [
    (Color::Black, [0, 0, 0]),
    (Color::DarkRed, [205, 0, 0]),
    (Color::DarkGreen, [0, 205, 0]),
    (Color::DarkYellow, [205, 205, 0]),
    (Color::DarkBlue, [0, 0, 238]),
    (Color::DarkMagenta, [205, 0, 205]),
    (Color::DarkCyan, [0, 205, 205]),
    (Color::Grey, [229, 229, 229]),
    (Color::DarkGrey, [127, 127, 127]),
    (Color::Red, [255, 0, 0]),
    (Color::Green, [0, 255, 0]),
    (Color::Yellow, [255, 255, 0]),
    (Color::Blue, [92, 92, 255]),
    (Color::Magenta, [255, 0, 255]),
    (Color::Cyan, [0, 255, 255]),
    (Color::White, [255, 255, 255]),
];

fn basic(colour: Rgba8) -> Color {
    BASIC
        .iter()
        .min_by_key(|(_, [red, green, blue])| distance(colour, *red, *green, *blue))
        .map_or(Color::Reset, |(color, _)| *color)
}
