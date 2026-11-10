use crossterm::style::Color as TermColor;

use super::paint;
use crate::style::{Color, Tone};
use crate::terminal::caps::ColorDepth;
use crate::terminal::paint::Ink;

const GREYS: [TermColor; 5] = [
    TermColor::Reset,
    TermColor::Black,
    TermColor::Grey,
    TermColor::DarkGrey,
    TermColor::White,
];

#[test]
fn tones_map_to_their_ansi_hue() {
    let table = [
        (Tone::Success, TermColor::Green),
        (Tone::Warning, TermColor::Yellow),
        (Tone::Error, TermColor::Red),
        (Tone::Info, TermColor::Cyan),
        (Tone::Primary, TermColor::Blue),
        (Tone::Neutral, TermColor::Reset),
    ];
    for (tone, color) in table {
        assert_eq!(paint(Ink::Of(tone), ColorDepth::Ansi16), color, "{tone:?}");
    }
}

#[test]
fn saturated_exact_colours_never_turn_grey() {
    let muted = [
        (74, 160, 90),
        (190, 70, 70),
        (200, 170, 60),
        (80, 120, 200),
        (60, 40, 110),
    ];
    for (r, g, b) in muted {
        let colour = Color { r, g, b, a: 255 };
        let basic = paint(Ink::Exact(colour), ColorDepth::Ansi16);
        assert!(!GREYS.contains(&basic), "{colour:?} -> {basic:?}");
        let indexed = paint(Ink::Exact(colour), ColorDepth::Ansi256);
        assert!(
            matches!(indexed, TermColor::AnsiValue(index) if index < 232),
            "{colour:?} -> {indexed:?}"
        );
    }
}

#[test]
fn neutral_exact_colours_stay_grey() {
    let colour = Color {
        r: 128,
        g: 128,
        b: 130,
        a: 255,
    };
    assert_eq!(
        paint(Ink::Exact(colour), ColorDepth::Ansi16),
        TermColor::DarkGrey
    );
}
