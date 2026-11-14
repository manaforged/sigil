use std::f32::consts::PI;

use sigil::animation::{Animator, Property, Spring};
use sigil::layout::{LayoutStyle, Position};
use sigil::prelude::*;

const CARD_WIDTH: f32 = 300.0;
const CARD_HEIGHT: f32 = 200.0;
const DEPTH: f32 = 900.0;
const DESK: Color = Color::rgb(46, 52, 48);
const CARD: Color = Color::rgb(250, 248, 240);
const INK: Color = Color::rgb(34, 38, 46);
const RULE: Color = Color::rgb(204, 78, 78);
const LINE: Token = Token::Custom("deck-rule");

struct Card {
    question: &'static str,
    answer: &'static str,
}

const CARDS: [Card; 3] = [
    Card {
        question: "What does get do inside a closure?",
        answer: "It subscribes that closure. peek reads without subscribing.",
    },
    Card {
        question: "Which setter sets size, line height, and weight together?",
        answer: ".text_role(TextRole::Title)",
    },
    Card {
        question: "What runs the same tree in the terminal?",
        answer: "Call .terminal() on the AppConfig that sigil::run takes.",
    },
];

/// The deck with every card face up.
#[must_use]
pub fn app() -> impl IntoNode {
    turned([0.0; 3])
}

/// The deck with each card turned by a share of a half turn, from 0 for the question to 1 for the answer.
#[must_use]
pub fn turned(turns: [f32; 3]) -> impl IntoNode {
    column()
        .theme(theme)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Space::Xxl)
        .gap(Space::Xl)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
        .background(Token::Surface)
        .child(
            text("Sigil in three cards")
                .text_role(TextRole::Heading)
                .text_color(Token::OnSurface),
        )
        .child(
            row()
                .gap(Space::Xl)
                .children(CARDS.iter().zip(turns).map(|(card, turn)| seat(card, turn))),
        )
}

fn theme() -> Theme {
    Theme::dark()
        .token(Token::Surface, DESK)
        .token(Token::Accent, RULE)
        .token(LINE, RULE)
}

fn seat(card: &'static Card, turn: f32) -> NodeBuilder {
    let angle = Animator::spring(turn * PI, Spring::BOUNCY);
    column()
        .gap(Space::Md)
        .align_items(AlignItems::Center)
        .child(
            container()
                .width(CARD_WIDTH)
                .height(CARD_HEIGHT)
                .perspective(DEPTH)
                .child(face(card.question, "Question").drive(Property::RotateY, angle.value()))
                .child(
                    face(card.answer, "Answer")
                        .drive(Property::RotateY, move || angle.get() + PI),
                ),
        )
        .child(button("Turn over").activate(move || {
            let answer = angle.value().peek() < PI / 2.0;
            angle.target(if answer { PI } else { 0.0 });
        }))
}

fn face(words: &'static str, side: &'static str) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .position(Position::Absolute)
                .inset(Length::Px(0.0)),
        )
        .backface_hidden()
        .radius(Radius::Md)
        .shadow(sigil::style::Elevation::Lg)
        .background(CARD)
        .child(
            column()
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(Space::Lg)
                .gap(Space::Md)
                .child(
                    text(side)
                        .text_role(TextRole::Caption)
                        .text_color(LINE),
                )
                .child(divider(Orientation::Horizontal).border_color(LINE))
                .child(text(words).text_role(TextRole::Title).text_color(INK)),
        )
}
