use std::time::Duration;

use sigil::node::TextBuilder;
use sigil::prelude::*;

use crate::sheet::{INK, PAPER, SIGNAL as MARK};

const HERO: f32 = 112.0;
const HERO_LINE: f32 = 100.0;
const NUMERAL: f32 = 88.0;
const NAME: f32 = 30.0;
const STRIP: f32 = 44.0;
const SMALL: f32 = 11.0;
const TRACKING: f32 = 0.14;
const LEFT: f32 = 560.0;
const NAME_WIDTH: f32 = 240.0;
const DRIFT: f32 = 28.0;
const STAGGER: Duration = Duration::from_millis(45);
const GLIDE: Duration = Duration::from_millis(280);
const WHY: &str = "a display size outside the type scale of an app";

struct Part {
    name: &'static str,
    kind: &'static str,
    note: &'static str,
}

const PARTS: [Part; 7] = [
    Part {
        name: "Wash",
        kind: "PAINT",
        note: "Pigment pooled over paper, drawn by the renderer.",
    },
    Part {
        name: "Backdrop",
        kind: "GLASS",
        note: "A panel that blurs what is already painted behind it.",
    },
    Part {
        name: "Spring",
        kind: "MOTION",
        note: "A value that keeps its speed when its target moves.",
    },
    Part {
        name: "Presence",
        kind: "ENTER, EXIT",
        note: "A row that says where it comes from and where it goes.",
    },
    Part {
        name: "Layout",
        kind: "GLIDE",
        note: "Neighbours slide to their new place when a row changes.",
    },
    Part {
        name: "Words",
        kind: "TYPE",
        note: "One node per word behind one sentence for a reader.",
    },
    Part {
        name: "Canvas",
        kind: "DRAW",
        note: "Paths, gradients and text drawn straight into the scene.",
    },
];

/// The typefaces of the index: a serif for the roman and its italic in the display slot.
#[must_use]
pub fn fonts() -> [(FontFamily, &'static [u8]); 2] {
    [
        (
            FontFamily::Serif,
            include_bytes!("../fonts/InstrumentSerif-Regular.ttf"),
        ),
        (
            FontFamily::Display,
            include_bytes!("../fonts/InstrumentSerif-Italic.ttf"),
        ),
    ]
}

/// An editorial index of parts with the third part open.
#[must_use]
pub fn app() -> impl IntoNode {
    open(2)
}

/// The index with the part at `at` open.
#[must_use]
pub fn open(at: usize) -> impl IntoNode {
    let active = signal(at);
    column()
        .theme(theme)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding_x(Space::Xxl)
        .padding_y(Space::Xl)
        .background(Token::Surface)
        .child(masthead())
        .child(
            row()
                .width(Length::Fill)
                .grow(1.0)
                .gap(Space::Xxl)
                .child(cover(active))
                .child(
                    column()
                        .grow(1.0)
                        .padding_top(Space::Lg)
                        .children((0..PARTS.len()).map(|at| entry(at, active))),
                ),
        )
        .child(strip())
}

fn theme() -> Theme {
    Theme::light()
        .token(Token::Surface, PAPER)
        .token(Token::OnSurface, INK)
        .token(Token::Border, INK)
}

fn small(label: impl Into<TextContent>) -> TextBuilder {
    text(label)
        .font_family(FontFamily::Mono)
        .font_size(SMALL, "a running head in small capitals")
        .letter_spacing(TRACKING)
}

fn masthead() -> NodeBuilder {
    row()
        .width(Length::Fill)
        .padding_bottom(Space::Md)
        .border_bottom(1)
        .border_color(Token::Border)
        .child(small("SIGIL  /  INDEX OF PARTS"))
        .child(spacer())
        .child(small("VOL. 0.2  /  NOVEMBER 2026"))
        .into()
}

fn display(line: &'static str, family: FontFamily) -> TextBuilder {
    text(line)
        .font_family(family)
        .font_size(HERO, WHY)
        .line_height(HERO_LINE)
        .font_weight(400)
        .letter_spacing(-0.03)
}

fn cover(active: Signal<usize>) -> NodeBuilder {
    let number = memo(move || active.get() + 1);
    column()
        .width(LEFT)
        .padding_top(Space::Xl)
        .child(display("Interfaces,", FontFamily::Serif))
        .child(display("set in type.", FontFamily::Display))
        .child(spacer())
        .child(
            row()
                .gap(Space::Lg)
                .align_items(AlignItems::End)
                .padding_bottom(Space::Lg)
                .child(
                    ticker(number)
                        .format(|number| format!("{number:02}"))
                        .font_family(FontFamily::Serif)
                        .font_size(NUMERAL, WHY)
                        .line_height(NUMERAL)
                        .font_weight(400),
                )
                .child(
                    column()
                        .gap(Space::Xs)
                        .padding_bottom(Space::Md)
                        .child(small(move || PARTS[active.get() % PARTS.len()].kind))
                        .child(for_each(move || vec![active.get()], |at| *at, |at| note(*at))),
                ),
        )
        .into()
}

fn note(at: usize) -> impl IntoNode {
    words(PARTS[at % PARTS.len()].note, |index, word| {
        text(word).animation(Keyframes::fade().delay(STAGGER * index))
    })
    .text_role(TextRole::Body)
}

fn entry(at: usize, active: Signal<usize>) -> NodeBuilder {
    let part = &PARTS[at];
    let hovered = signal(false);
    effect(move || {
        if hovered.get() {
            active.set(at);
        }
    });
    let chosen = move || active.get() == at;
    row()
        .width(Length::Fill)
        .gap(Space::Lg)
        .align_items(AlignItems::Center)
        .padding_x(Space::Md)
        .padding_y(Space::Sm)
        .border_bottom(1)
        .border_color(Token::Border)
        .transition(Transition::new().duration(GLIDE).ease(Easing::CubicOut).layout())
        .paint_when(chosen, |style| style.background(MARK))
        .track_hovered(hovered)
        .pointer_up(move |_| active.set(at))
        .accessible(part.name)
        .child(small(format!("{:02}", at + 1)))
        .child(
            text(part.name)
                .font_family(FontFamily::Serif)
                .font_size(NAME, WHY)
                .font_weight(400)
                .width(NAME_WIDTH),
        )
        .child(spacer())
        .child(small(part.kind))
        .into()
}

fn strip() -> NodeBuilder {
    container()
        .width(Length::Fill)
        .padding_top(Space::Md)
        .border_top(1)
        .border_color(Token::Border)
        .child(marquee(names).speed(DRIFT).width(Length::Fill))
}

fn names() -> NodeBuilder {
    row()
        .gap(Space::Xl)
        .align_items(AlignItems::Center)
        .children(PARTS.iter().flat_map(|part| {
            [
                text(part.name)
                    .font_family(FontFamily::Display)
                    .font_size(STRIP, WHY)
                    .line_height(STRIP)
                    .font_weight(400),
                small("+"),
            ]
        }))
        .into()
}
