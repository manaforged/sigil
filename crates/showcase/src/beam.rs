use std::time::Duration;

use sigil::animation::{Property, clock};
use sigil::prelude::*;
use sigil::style::Conic;

use crate::focus::theme;

const WIDTH: f32 = 620.0;
const TURN: f64 = 90.0;
const STAGGER: Duration = Duration::from_millis(70);
const SESSION: u32 = 25;
const LOGGED: u32 = 175;
const HEADLINE: &str = "Every pixel here is a Rust function";

/// The beam card with its border light turning on the animation clock.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "an angle under 360 degrees fits in f32"
)]
pub fn app() -> impl IntoNode {
    let seconds = clock();
    lit(move || ((seconds.get() * TURN) % 360.0) as f32)
}

/// The beam card with its border light at `angle` degrees clockwise from up.
#[must_use]
pub fn lit(angle: impl Reactive<f32>) -> impl IntoNode {
    let minutes = signal(LOGGED);
    column()
        .theme(theme)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
        .background(Token::Surface)
        .child(
            container()
                .width(WIDTH)
                .padding(Space::Xxs)
                .radius(Radius::Lg)
                .background(
                    Background::conic(Conic::new(0))
                        .start(Token::Accent)
                        .end(Token::Surface),
                )
                .drive(Property::GradientAngle, angle)
                .child(card(minutes)),
        )
}

fn card(minutes: Signal<u32>) -> NodeBuilder {
    column()
        .width(Length::Fill)
        .padding(Space::Xxl)
        .gap(Space::Xl)
        .radius(Radius::Lg)
        .background(Token::Surface)
        .child(
            words(HEADLINE, |index, word| {
                let wait = STAGGER.saturating_mul(u32::try_from(index).unwrap_or(u32::MAX));
                text(word).animation(Keyframes::fade().delay(wait))
            })
            .text_role(TextRole::Title),
        )
        .child(
            row()
                .width(Length::Fill)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween)
                .child(
                    ticker(minutes)
                        .format(|minutes| format!("{minutes} min focused"))
                        .text_role(TextRole::Heading),
                )
                .child(
                    button("Log a session")
                        .primary()
                        .activate(move || minutes.update(|minutes| *minutes += SESSION)),
                ),
        )
        .into()
}
