pub(crate) mod sky;
pub(crate) mod sun;

use sigil::animation::{Animator, Spring};
use sigil::prelude::*;
use sigil::style::Backdrop;

use crate::overlay;
use sun::{Sun, clock, span};

pub(crate) const PLACE: &str = "Austin, Texas";
pub(crate) const LATITUDE: f32 = 30.27;
pub(crate) const START_DAY: f32 = 317.0;
const START_HOUR: f32 = 16.5;
const HOURS: f32 = 24.0;
const DAYS: f32 = 365.0;
const HOUR_MARKS: [&str; 5] = ["00", "06", "12", "18", "24"];
const DAY_MARKS: [&str; 5] = ["1", "92", "183", "274", "365"];
const HERO_SIZE: f32 = 132.0;
const SKY_SHARE: f32 = 0.58;
const DAY_WIDTH: f32 = 280.0;

#[derive(Clone, Copy)]
struct Almanac {
    hour: Signal<f32>,
    day: Signal<f32>,
    drift: Animator,
    sun: Signal<Sun>,
}

impl Almanac {
    fn altitude(self) -> f32 {
        self.sun.get().altitude(self.drift.get())
    }
}

/// The almanac page late in the afternoon.
#[must_use]
pub fn app() -> impl IntoNode {
    at(START_HOUR)
}

/// The almanac page at `hour`: a painted sky with the day's times set on it.
#[must_use]
pub fn at(hour: f32) -> impl IntoNode {
    let day = signal(START_DAY);
    let almanac = Almanac {
        hour: signal(hour),
        day,
        drift: Animator::spring(hour, Spring::GENTLE),
        sun: memo(move || Sun::new(LATITUDE, day.get())),
    };
    let night = memo(move || almanac.altitude() < 0.0);

    container()
        .theme(move || theme(night.get()))
        .width(Length::Fill)
        .height(Length::Fill)
        .child(sky(almanac))
        .child(
            column()
                .layout(overlay())
                .child(masthead(almanac))
                .child(controls(almanac)),
        )
}

fn theme(night: bool) -> Theme {
    let (backdrop, palette) = if night {
        (Backdrop::Dark, sky::NIGHT)
    } else {
        (Backdrop::Light, sky::DAY)
    };
    Theme::for_backdrop(backdrop)
        .token(Token::Surface, palette.paper)
        .token(Token::OnSurface, palette.ink)
        .token(Token::Accent, palette.disc)
}

fn land_theme() -> Theme {
    Theme::dark()
        .token(Token::Surface, sky::NIGHT.land)
        .token(Token::Accent, sky::DAY.disc)
}

fn sky(almanac: Almanac) -> impl IntoNode {
    canvas(move |scene, rect, theme| {
        sky::paint(
            scene,
            rect,
            theme,
            almanac.sun.peek(),
            almanac.drift.value().peek(),
        );
    })
    .redraw_on(move || (almanac.drift.get(), almanac.sun.get()))
    .width(Length::Fill)
    .height(Length::Fill)
}

fn masthead(almanac: Almanac) -> NodeBuilder {
    row()
        .width(Length::Fill)
        .height(Length::Percent(SKY_SHARE))
        .padding(Space::Xxl)
        .justify_content(JustifyContent::SpaceBetween)
        .align_items(AlignItems::Start)
        .child(reading(almanac))
        .child(place(almanac))
}

fn caption(words: impl Into<TextContent>) -> impl IntoNode {
    text(words)
        .text_role(TextRole::Caption)
        .font_weight(600)
        .letter_spacing(0.16)
        .text_color(Token::OnSurface)
}

fn reading(almanac: Almanac) -> NodeBuilder {
    column()
        .gap(Space::Xs)
        .child(caption("ALMANAC"))
        .child(
            text(move || clock(almanac.hour.get()))
                .font_size(HERO_SIZE, "the one numeral the page is read by")
                .line_height(HERO_SIZE)
                .font_weight(600)
                .letter_spacing(-0.035)
                .tabular_nums(true)
                .text_color(Token::OnSurface),
        )
        .child(
            text(move || {
                let altitude = almanac.altitude();
                let side = if altitude < 0.0 { "below" } else { "above" };
                format!("Solar time. Sun {:.0}° {side} the horizon.", altitude.abs())
            })
            .text_role(TextRole::Subtitle)
            .text_color(Token::OnSurface),
        )
}

fn place(almanac: Almanac) -> NodeBuilder {
    let (sun, day) = (almanac.sun, almanac.day);
    column()
        .gap(Space::Xs)
        .align_items(AlignItems::End)
        .child(caption(PLACE.to_uppercase()))
        .child(fact(format!("{LATITUDE}° N")))
        .child(fact(move || format!("Day {:.0} of {DAYS:.0}", day.get())))
        .child(fact(move || {
            format!("{} of daylight", span(sun.get().daylight()))
        }))
}

fn fact(words: impl Into<TextContent>) -> impl IntoNode {
    text(words)
        .text_role(TextRole::Body)
        .tabular_nums(true)
        .text_color(Token::OnSurface)
}

fn controls(almanac: Almanac) -> NodeBuilder {
    row()
        .theme(land_theme)
        .width(Length::Fill)
        .grow(1.0)
        .padding(Space::Xxl)
        .gap(Space::Xxl)
        .align_items(AlignItems::End)
        .child(
            slider(almanac.hour, 0.0..=HOURS)
                .label("Hour")
                .marks(HOUR_MARKS)
                .step(0.25)
                .change(move |hour| almanac.drift.target(hour))
                .grow(1.0),
        )
        .child(
            slider(almanac.day, 1.0..=DAYS)
                .label("Day")
                .marks(DAY_MARKS)
                .step(1.0)
                .width(DAY_WIDTH),
        )
}
