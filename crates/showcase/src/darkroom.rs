use std::time::Duration;

use sigil::animation::{Animator, Property};
use sigil::prelude::*;
use sigil::scene::BlendMode;
use sigil::style::{GradientAxis, MaskImage, Radial};

use crate::almanac::sky::{self, DAY};
use crate::almanac::sun::Sun;
use crate::almanac::{LATITUDE, PLACE, START_DAY};
use crate::overlay;

const PLATE_HOUR: f32 = 17.1;
const PANEL_WIDTH: f32 = 340.0;
const FADE: Duration = Duration::from_millis(320);
const SURROUND: Color = Color::rgb(24, 24, 24);
const SAFELIGHT: Color = Color::rgb(214, 72, 48);

struct Adjustment {
    label: &'static str,
    low: f32,
    high: f32,
    rest: f32,
    step: f32,
    property: fn(f32) -> Property,
}

const ADJUSTMENTS: [Adjustment; 6] = [
    Adjustment {
        label: "Exposure",
        low: 0.5,
        high: 1.5,
        rest: 1.0,
        step: 0.01,
        property: Property::Brightness,
    },
    Adjustment {
        label: "Contrast",
        low: 0.5,
        high: 1.5,
        rest: 1.0,
        step: 0.01,
        property: Property::Contrast,
    },
    Adjustment {
        label: "Saturation",
        low: 0.0,
        high: 2.0,
        rest: 1.0,
        step: 0.01,
        property: Property::Saturate,
    },
    Adjustment {
        label: "Warmth",
        low: 0.0,
        high: 1.0,
        rest: 0.0,
        step: 0.01,
        property: Property::Sepia,
    },
    Adjustment {
        label: "Hue",
        low: -180.0,
        high: 180.0,
        rest: 0.0,
        step: 1.0,
        property: Property::HueRotate,
    },
    Adjustment {
        label: "Soften",
        low: 0.0,
        high: 12.0,
        rest: 0.0,
        step: 0.1,
        property: Property::Blur,
    },
];

#[derive(Clone)]
struct Darkroom {
    values: Vec<Signal<f32>>,
    monochrome: Signal<bool>,
    grey: Animator,
    vignette: Signal<bool>,
    blend: Signal<usize>,
}

/// The darkroom screen with every adjustment at rest.
#[must_use]
pub fn app() -> impl IntoNode {
    graded(&[])
}

/// The darkroom screen with `values` in place of the first adjustments, a vignette, and a multiply leak.
#[must_use]
pub fn graded(values: &[f32]) -> impl IntoNode + use<> {
    let graded = !values.is_empty();
    let darkroom = Darkroom {
        values: ADJUSTMENTS
            .iter()
            .enumerate()
            .map(|(index, adjustment)| {
                signal(values.get(index).copied().unwrap_or(adjustment.rest))
            })
            .collect(),
        monochrome: signal(false),
        grey: Animator::tween(0.0, Transition::new().duration(FADE).ease(Easing::CubicOut)),
        vignette: signal(graded),
        blend: signal(usize::from(graded)),
    };
    let (monochrome, grey) = (darkroom.monochrome, darkroom.grey);
    effect(move || grey.target(if monochrome.get() { 1.0 } else { 0.0 }));

    row()
        .theme(theme)
        .width(Length::Fill)
        .height(Length::Fill)
        .background(Token::Surface)
        .child(stage(&darkroom))
        .child(panel(&darkroom))
}

fn theme() -> Theme {
    Theme::dark()
        .token(Token::Surface, SURROUND)
        .token(Token::Accent, SAFELIGHT)
}

fn stage(darkroom: &Darkroom) -> NodeBuilder {
    let darkroom = darkroom.clone();
    container()
        .grow(1.0)
        .height(Length::Fill)
        .padding(Space::Xxl)
        .child(
            container()
                .width(Length::Fill)
                .height(Length::Fill)
                .children_reactive(move || {
                    let mode = BlendMode::ALL
                        .get(darkroom.blend.get())
                        .copied()
                        .unwrap_or_default();
                    [plate(&darkroom), leak(mode)]
                }),
        )
}

fn plate(darkroom: &Darkroom) -> NodeBuilder {
    let sun = Sun::new(LATITUDE, START_DAY);
    let mut plate =
        canvas(move |scene, rect, theme| sky::paint(scene, rect, theme, sun, PLATE_HOUR))
            .width(Length::Fill)
            .height(Length::Fill)
            .drive(Property::Grayscale, darkroom.grey.value());
    for (adjustment, value) in ADJUSTMENTS.iter().zip(&darkroom.values) {
        plate = plate.drive(adjustment.property, *value);
    }
    if darkroom.vignette.get() {
        plate = plate.mask_image(MaskImage::Radial(Radial::new()));
    }
    plate.into()
}

fn leak(mode: BlendMode) -> NodeBuilder {
    container()
        .layout(overlay())
        .background(
            Background::linear(GradientAxis::LeftToRight)
                .start(DAY.pigments[1])
                .end(Color::TRANSPARENT),
        )
        .blend(mode)
}

fn panel(darkroom: &Darkroom) -> NodeBuilder {
    let reset = darkroom.clone();
    column()
        .width(PANEL_WIDTH)
        .height(Length::Fill)
        .padding(Space::Xl)
        .gap(Space::Lg)
        .child(
            column()
                .gap(Space::Xs)
                .child(text("Darkroom").text_role(TextRole::Heading))
                .child(
                    text(format!("Plate: dusk over {PLACE}, day {START_DAY}"))
                        .text_role(TextRole::Caption)
                        .text_color(Token::OnSurfaceMuted),
                ),
        )
        .child(
            column()
                .gap(Space::Md)
                .children(
                    ADJUSTMENTS
                        .iter()
                        .zip(&darkroom.values)
                        .map(|(adjustment, value)| {
                            slider(*value, adjustment.low..=adjustment.high)
                                .label(adjustment.label)
                                .step(adjustment.step)
                        }),
                ),
        )
        .child(toggle(darkroom.monochrome).label("Monochrome"))
        .child(toggle(darkroom.vignette).label("Vignette"))
        .child(
            dropdown(
                darkroom.blend,
                BlendMode::ALL.iter().map(|mode| format!("{mode:?}")),
            )
            .accessible("Light leak blend mode"),
        )
        .child(spacer())
        .child(button("Reset").activate(move || {
            for (adjustment, value) in ADJUSTMENTS.iter().zip(&reset.values) {
                value.set(adjustment.rest);
            }
            reset.monochrome.set(false);
            reset.vignette.set(false);
            reset.blend.set(0);
        }))
}
