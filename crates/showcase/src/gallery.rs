use sigil::animation::Property;
use sigil::prelude::*;
use sigil::scene::Shadow;
use sigil::style::FontFamily;

const STAGE: Color = Color::rgb(8, 9, 16);
const PANEL: Color = Color::rgb(18, 20, 32);
const WIDTH: f32 = 560.0;
const BLUR: f32 = 14.0;
const SPREAD: f32 = 0.0;
const RISE: f32 = 28.0;

struct Entry {
    title: &'static str,
    call: &'static str,
    light: Color,
}

const ENTRIES: [Entry; 8] = [
    Entry {
        title: "Glass that blurs what is behind it",
        call: ".backdrop_blur(26.0)",
        light: Color::rgb(86, 120, 255),
    },
    Entry {
        title: "A fling that lands on a stop",
        call: "drop.fling_to(event.velocity().y, &STOPS)",
        light: Color::rgb(255, 92, 122),
    },
    Entry {
        title: "Two faces in one 3D space",
        call: ".preserve_3d()",
        light: Color::rgb(34, 211, 170),
    },
    Entry {
        title: "Rows that enter and leave",
        call: ".exit(Property::Opacity(0.0))",
        light: Color::rgb(255, 196, 64),
    },
    Entry {
        title: "A border that turns",
        call: ".drive(Property::GradientAngle, angle)",
        light: Color::rgb(190, 110, 255),
    },
    Entry {
        title: "Words that arrive one by one",
        call: "words(headline, |index, word| ..)",
        light: Color::rgb(86, 200, 255),
    },
    Entry {
        title: "A glow on any node",
        call: ".shadow(Shadow::glow(color, 14.0, 0.0))",
        light: Color::rgb(255, 120, 60),
    },
    Entry {
        title: "Cards that know they are in view",
        call: ".track_visible(seen)",
        light: Color::rgb(120, 230, 110),
    },
];

/// The gallery scrolled to the top.
#[must_use]
pub fn app() -> impl IntoNode {
    at(0.0)
}

/// The gallery scrolled down by `offset` pixels.
#[must_use]
pub fn at(offset: f32) -> impl IntoNode {
    let list = column()
        .width(Length::Fill)
        .padding(Space::Xxl)
        .gap(Space::Xl)
        .align_items(AlignItems::Center)
        .child(
            text("New in Sigil this week")
                .text_role(TextRole::Display)
                .text_color(Token::OnSurface),
        )
        .children(ENTRIES.iter().map(entry));
    container()
        .theme(theme)
        .width(Length::Fill)
        .height(Length::Fill)
        .background(Token::Surface)
        .child(
            scroll(list)
                .scroll_y(signal(offset))
                .width(Length::Fill)
                .height(Length::Fill),
        )
}

fn theme() -> Theme {
    Theme::dark().token(Token::Surface, STAGE)
}

fn entry(entry: &'static Entry) -> NodeBuilder {
    let seen = signal(0.0_f32);
    let panel = column()
        .width(WIDTH)
        .padding(Space::Xl)
        .gap(Space::Sm)
        .radius(Radius::Lg)
        .background(PANEL)
        .border_width(1)
        .border_color(entry.light)
        .shadow(Shadow::glow(entry.light, BLUR, SPREAD))
        .drive(Property::Opacity, seen)
        .drive(Property::TranslateY, move || (1.0 - seen.get()) * RISE)
        .child(text(entry.title).text_role(TextRole::Title))
        .child(
            text(entry.call)
                .font_family(FontFamily::Mono)
                .text_color(Token::OnSurfaceMuted),
        );
    container().track_visible(seen).child(panel).into()
}
