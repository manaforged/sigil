use sigil::node::TextBuilder;
use sigil::prelude::*;

/// The paper every figure of the sheet is printed on.
pub const PAPER: Color = Color::rgb(237, 237, 232);
/// The ink of rules, labels and type.
pub const INK: Color = Color::rgb(17, 17, 17);
/// The grey of secondary rules.
pub const RULE: Color = Color::rgb(185, 185, 178);
/// The one signal colour, kept for live values.
pub const SIGNAL: Color = Color::rgb(252, 79, 0);

const TRACKING: f32 = 0.12;

/// The faces of the sheet: Host Grotesk for type and Martian Mono for labels and readouts.
#[must_use]
pub fn fonts() -> [(FontFamily, &'static [u8]); 2] {
    [
        (
            FontFamily::Sans,
            include_bytes!("../fonts/HostGrotesk.ttf").as_slice(),
        ),
        (
            FontFamily::Mono,
            include_bytes!("../fonts/MartianMono.ttf").as_slice(),
        ),
    ]
}

/// The theme of a figure: paper, ink and the signal colour.
#[must_use]
pub fn theme() -> Theme {
    Theme::light()
        .token(Token::Surface, PAPER)
        .token(Token::OnSurface, INK)
        .token(Token::Accent, SIGNAL)
        .token(Token::AccentHover, SIGNAL)
}

pub(crate) fn label(words: impl Into<TextContent>) -> TextBuilder {
    text(words)
        .text_role(TextRole::Caption)
        .font_family(FontFamily::Mono)
        .letter_spacing(TRACKING)
        .text_color(INK)
}

pub(crate) fn mix(from: Color, to: Color, share: f32) -> Color {
    let channel = |a: u8, b: u8| {
        let value = (f32::from(b) - f32::from(a)).mul_add(share.clamp(0.0, 1.0), f32::from(a));
        value.round().clamp(0.0, 255.0) as u8
    };
    Color::rgba(
        channel(from.r, to.r),
        channel(from.g, to.g),
        channel(from.b, to.b),
        channel(from.a, to.a),
    )
}
