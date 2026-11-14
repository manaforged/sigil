use std::time::Duration;

use sigil::prelude::*;
use sigil::scene::{DrawRect, SceneBuilder};

use crate::overlay;

const STAGE: Color = Color::rgb(10, 9, 22);
const GLASS: Color = Color::rgba(14, 12, 30, 120);
const ACCENT: Color = Color::rgb(196, 38, 78);
const RIM: Color = Color::rgba(255, 255, 255, 64);
const SPECK: Color = Color::rgba(255, 255, 255, 150);
pub(crate) const BLUR: f32 = 26.0;
const SATURATE: f32 = 1.7;
const BRIGHTNESS: f32 = 1.08;
const CARD_WIDTH: f32 = 440.0;
const HERO_SIZE: f32 = 104.0;
const SPECK_PITCH: f32 = 28.0;
const SPECK_SIZE: f32 = 1.5;
const START_MINUTES: f32 = 25.0;
const MINUTES: std::ops::RangeInclusive<f32> = 5.0..=60.0;
const SECOND: Duration = Duration::from_secs(1);

struct Light {
    at: [f32; 2],
    reach: f32,
    color: Color,
}

const LIGHTS: [Light; 4] = [
    Light {
        at: [0.24, 0.30],
        reach: 0.42,
        color: Color::rgb(255, 92, 122),
    },
    Light {
        at: [0.70, 0.22],
        reach: 0.38,
        color: Color::rgb(86, 120, 255),
    },
    Light {
        at: [0.58, 0.78],
        reach: 0.46,
        color: Color::rgb(34, 211, 170),
    },
    Light {
        at: [0.36, 0.66],
        reach: 0.24,
        color: Color::rgb(255, 196, 64),
    },
];

#[derive(Clone, Copy)]
struct Focus {
    minutes: Signal<f32>,
    left: Signal<u32>,
    running: Signal<bool>,
}

/// The focus timer on frosted glass over stage lights.
#[must_use]
pub fn app() -> impl IntoNode {
    frosted(BLUR)
}

/// The focus timer with the glass blurring what is behind it by `blur` pixels.
#[must_use]
pub fn frosted(blur: f32) -> impl IntoNode {
    let focus = Focus {
        minutes: signal(START_MINUTES),
        left: signal(seconds(START_MINUTES)),
        running: signal(false),
    };
    interval(SECOND, move || {
        if focus.running.peek() {
            focus.left.update(|left| *left = left.saturating_sub(1));
        }
    });

    container()
        .theme(theme)
        .width(Length::Fill)
        .height(Length::Fill)
        .child(canvas(stage).width(Length::Fill).height(Length::Fill))
        .child(
            column()
                .layout(overlay())
                .justify_content(JustifyContent::Center)
                .align_items(AlignItems::Center)
                .child(card(focus, blur)),
        )
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a session is a small positive number of seconds"
)]
fn seconds(minutes: f32) -> u32 {
    (minutes * 60.0).round() as u32
}

pub(crate) fn theme() -> Theme {
    Theme::dark()
        .token(Token::Surface, STAGE)
        .token(Token::Accent, ACCENT)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a canvas is a small positive number of pixels on each side"
)]
pub(crate) fn stage(scene: &mut SceneBuilder, rect: Rect, theme: &Theme) {
    scene.draw_rect(DrawRect::new(rect, STAGE));
    let (width, height) = (rect.width, rect.height);
    for light in &LIGHTS {
        let centre = (light.at[0] * width, light.at[1] * height);
        let stops = vec![(0.0, light.color), (1.0, light.color.alpha(0))];
        path(
            width.round().max(1.0) as u32,
            height.round().max(1.0) as u32,
        )
        .fill_radial(centre, light.reach * width, stops)
        .move_to(0.0, 0.0)
        .line_to(width, 0.0)
        .line_to(width, height)
        .line_to(0.0, height)
        .close()
        .draw(scene, rect, theme);
    }
    let (columns, rows) = ((width / SPECK_PITCH) as u32, (height / SPECK_PITCH) as u32);
    for row in 1..rows {
        for column in 1..columns {
            let at = Point::new(
                (column as f32).mul_add(SPECK_PITCH, rect.origin.x) - SPECK_SIZE,
                (row as f32).mul_add(SPECK_PITCH, rect.origin.y) - SPECK_SIZE,
            );
            let bounds = Rect::new(at, SPECK_SIZE * 2.0, SPECK_SIZE * 2.0);
            scene.draw_rect(DrawRect::new(bounds, SPECK).radius(SPECK_SIZE));
        }
    }
}

pub(crate) fn glass<N: Paint>(node: N, blur: f32) -> N {
    node.background(GLASS)
        .border_width(1)
        .border_color(RIM)
        .radius(Radius::Lg)
        .backdrop_blur(blur)
        .backdrop_saturate(SATURATE)
        .backdrop_brightness(BRIGHTNESS)
}

fn card(focus: Focus, blur: f32) -> NodeBuilder {
    let card = column()
        .width(CARD_WIDTH)
        .padding(Space::Xxl)
        .gap(Space::Lg)
        .align_items(AlignItems::Center);
    glass(card, blur)
        .child(
            text("FOCUS")
                .text_role(TextRole::Caption)
                .font_weight(600)
                .letter_spacing(0.16),
        )
        .child(
            text(move || {
                let left = focus.left.get();
                format!("{:02}:{:02}", left / 60, left % 60)
            })
            .font_size(HERO_SIZE, "the one numeral the timer is read by")
            .line_height(HERO_SIZE)
            .font_weight(600)
            .letter_spacing(-0.03)
            .tabular_nums(true),
        )
        .child(
            slider(focus.minutes, MINUTES)
                .label("Minutes")
                .step(5.0)
                .change(move |minutes| focus.left.set(seconds(minutes)))
                .width(Length::Fill),
        )
        .child(
            row()
                .gap(Space::Md)
                .child(
                    button(move || {
                        if focus.running.get() {
                            "Pause"
                        } else {
                            "Start"
                        }
                    })
                    .primary()
                    .activate(move || focus.running.update(|running| *running = !*running)),
                )
                .child(button("Reset").activate(move || {
                    focus.running.set(false);
                    focus.left.set(seconds(focus.minutes.peek()));
                })),
        )
        .into()
}
