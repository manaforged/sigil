use sigil::prelude::*;
use sigil::scene::{DrawRect, SceneBuilder, Wash};
use sigil::style::TextStyle;
use sigil::widgets::PathBuilder;

use super::sun::{Sun, clock};

/// The share of the page above the horizon line, out of 100.
pub(crate) const HORIZON: f32 = 70.0;
const ZENITH_RISE: f32 = 58.0;
const UNITS: f32 = 100.0;
const HOURS: f32 = 24.0;
const TRACK_STEPS: u8 = 96;
const TWILIGHT_DEGREES: f32 = 6.0;
const GOLDEN_PEAK_DEGREES: f32 = 3.0;
const GOLDEN_WIDTH_DEGREES: f32 = 13.0;
const DISC_RADIUS: f32 = 19.0;
const HALO_RADIUS: f32 = 250.0;
const HALO: [(f32, u8); 4] = [(0.0, 235), (0.1, 150), (0.42, 46), (1.0, 0)];
const LABEL_CLEARANCE_HOURS: f32 = 0.8;
const WASH_SEED: u32 = 317;
const LABEL_HOURS: [f32; 5] = [6.0, 9.0, 12.0, 15.0, 18.0];
const LABEL_SIZE: f32 = 11.0;
const LABEL_LIFT: f32 = 22.0;
const DOT: f32 = 1.5;
const LABEL_GAP: f32 = 10.0;

/// The paints for one end of the day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Palette {
    /// The paper under the wash.
    pub(crate) paper: Color,
    /// The pigments pooled over the paper.
    pub(crate) pigments: [Color; 3],
    /// The sun's disc.
    pub(crate) disc: Color,
    /// The light that gathers along the horizon when the sun is low.
    pub(crate) glow: Color,
    /// The nearest ridge.
    pub(crate) land: Color,
    /// Lines and labels drawn over the sky.
    pub(crate) ink: Color,
}

/// Cerulean blue, cobalt, and raw sienna on cold-press paper.
pub(crate) const DAY: Palette = Palette {
    paper: Color::rgb(244, 239, 226),
    pigments: [
        Color::rgb(96, 156, 204),
        Color::rgb(142, 184, 214),
        Color::rgb(232, 200, 150),
    ],
    disc: Color::rgb(250, 196, 92),
    glow: Color::rgb(244, 150, 88),
    land: Color::rgb(34, 46, 44),
    ink: Color::rgb(38, 46, 62),
};

/// Indigo, dioxazine violet, and viridian over Payne's grey.
pub(crate) const NIGHT: Palette = Palette {
    paper: Color::rgb(11, 14, 26),
    pigments: [
        Color::rgb(40, 58, 124),
        Color::rgb(92, 58, 130),
        Color::rgb(30, 104, 112),
    ],
    disc: Color::rgb(222, 92, 54),
    glow: Color::rgb(170, 70, 96),
    land: Color::rgb(6, 8, 15),
    ink: Color::rgb(176, 186, 214),
};

impl Palette {
    fn mix(self, other: Self, amount: f32) -> Self {
        let [a, b, c] = self.pigments;
        let [x, y, z] = other.pigments;
        Self {
            paper: blend(self.paper, other.paper, amount),
            pigments: [
                blend(a, x, amount),
                blend(b, y, amount),
                blend(c, z, amount),
            ],
            disc: blend(self.disc, other.disc, amount),
            glow: blend(self.glow, other.glow, amount),
            land: blend(self.land, other.land, amount),
            ink: blend(self.ink, other.ink, amount),
        }
    }
}

fn blend(from: Color, to: Color, amount: f32) -> Color {
    Color::rgb(
        channel(from.r, to.r, amount),
        channel(from.g, to.g, amount),
        channel(from.b, to.b, amount),
    )
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=255 before the cast"
)]
fn byte(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

fn channel(from: u8, to: u8, amount: f32) -> u8 {
    let (from, to) = (f32::from(from), f32::from(to));
    byte((to - from).mul_add(amount, from))
}

/// How much of the day's light is left at `altitude` degrees, from 0 at night to 1 in daylight.
#[must_use]
pub(crate) fn daylight(altitude: f32) -> f32 {
    let amount = ((altitude + TWILIGHT_DEGREES) / (TWILIGHT_DEGREES * 2.0)).clamp(0.0, 1.0);
    amount * amount * 2.0f32.mul_add(-amount, 3.0)
}

fn golden(altitude: f32) -> f32 {
    let off = (altitude - GOLDEN_PEAK_DEGREES) / GOLDEN_WIDTH_DEGREES;
    (-off * off).exp()
}

struct Page<'a> {
    scene: &'a mut SceneBuilder,
    rect: Rect,
    theme: &'a Theme,
    palette: Palette,
    sun: Sun,
    warmth: f32,
}

impl Page<'_> {
    fn point(&self, hour: f32) -> Point {
        let x = (hour / HOURS).mul_add(self.rect.width, self.rect.origin.x);
        let y =
            (height(self.sun.altitude(hour)) / UNITS).mul_add(self.rect.height, self.rect.origin.y);
        Point::new(x, y)
    }

    fn label(&mut self, at: Point, words: &str, side: f32) {
        let style = TextStyle::default()
            .font_size(LABEL_SIZE)
            .font_weight(500)
            .letter_spacing_em(0.06)
            .tabular_nums(true);
        let size = self.scene.measure_text(words, &style);
        let reach = size.width / 2.0 + LABEL_GAP;
        let lift = if side == 0.0 {
            LABEL_LIFT
        } else {
            size.height / 2.0
        };
        let origin = Point::new(side.mul_add(reach, at.x - size.width / 2.0), at.y - lift);
        self.scene
            .draw_text(origin, words, &style, self.palette.ink.alpha(210));
    }

    fn dot(&mut self, at: Point) {
        let bounds = Rect::new(Point::new(at.x - DOT, at.y - DOT), DOT * 2.0, DOT * 2.0);
        self.scene
            .draw_rect(DrawRect::new(bounds, self.palette.ink.alpha(200)).radius(DOT));
    }
}

/// Paints the sky, the sun on its track, and the land for `sun` at `hour`.
pub(crate) fn paint(scene: &mut SceneBuilder, rect: Rect, theme: &Theme, sun: Sun, hour: f32) {
    let altitude = sun.altitude(hour);
    let light = daylight(altitude);
    let mut page = Page {
        scene,
        rect,
        theme,
        palette: NIGHT.mix(DAY, light),
        sun,
        warmth: golden(altitude),
    };
    page.scene.draw_wash(rect, wash(page.palette, rect, light));
    glaze(page.palette, golden(altitude)).draw(page.scene, rect, theme);
    track(sun, page.palette).draw(page.scene, rect, theme);
    marks(&mut page, hour);
    disc(&mut page, hour);
    ridges(&mut page);
}

fn wash(palette: Palette, rect: Rect, light: f32) -> Wash {
    let [first, second, third] = palette.pigments;
    Wash::new(palette.paper)
        .pigment(first)
        .pigment(second)
        .pigment(third)
        .seed(WASH_SEED)
        .scale(rect.width * 0.8)
        .density(0.75f32.mul_add(light, 0.95))
        .pools(if light > 0.5 { 4 } else { 0 })
        .edges(0.45 * light)
        .granulation(0.6 * light)
        .grain(0.7 * light)
        .specks(0.72f32.mul_add(-light, 0.75))
        .band(light < 0.5)
}

fn glaze(palette: Palette, strength: f32) -> PathBuilder {
    let peak = byte(strength * 215.0);
    path(100, 100)
        .fill_linear(
            (0.0, 0.0),
            (0.0, HORIZON + 6.0),
            vec![
                (0.0, palette.glow.alpha(0)),
                (0.45, palette.glow.alpha(peak / 5)),
                (1.0, palette.glow.alpha(peak)),
            ],
        )
        .move_to(0.0, 0.0)
        .line_to(UNITS, 0.0)
        .line_to(UNITS, UNITS)
        .line_to(0.0, UNITS)
        .close()
}

fn height(altitude: f32) -> f32 {
    (altitude / 90.0).mul_add(-ZENITH_RISE, HORIZON)
}

fn track(sun: Sun, palette: Palette) -> PathBuilder {
    let mut line = path(100, 100)
        .stroke(palette.ink.alpha(120), 1.0)
        .dash(&[1.0, 5.0])
        .non_scaling_stroke();
    for step in 0..=TRACK_STEPS {
        let hour = f32::from(step) / f32::from(TRACK_STEPS) * HOURS;
        let (x, y) = (hour / HOURS * UNITS, height(sun.altitude(hour)));
        line = if step == 0 {
            line.move_to(x, y)
        } else {
            line.line_to(x, y)
        };
    }
    line
}

fn marks(page: &mut Page<'_>, now: f32) {
    let (rise, set) = (page.sun.sunrise(), page.sun.sunset());
    for hour in LABEL_HOURS {
        let clear = (hour - now).abs() > LABEL_CLEARANCE_HOURS;
        if clear && hour > rise + 0.5 && hour < set - 0.5 {
            let at = page.point(hour);
            page.dot(at);
            page.label(at, &clock(hour), 0.0);
        }
    }
    for (hour, name, side) in [(rise, "rise", -1.0), (set, "set", 1.0)] {
        let at = page.point(hour);
        page.dot(at);
        page.label(at, &format!("{name} {}", clock(hour)), side);
    }
}

fn disc(page: &mut Page<'_>, hour: f32) {
    let at = page.point(hour);
    let bounds = Rect::new(
        Point::new(at.x - DISC_RADIUS, at.y - DISC_RADIUS),
        DISC_RADIUS * 2.0,
        DISC_RADIUS * 2.0,
    );
    halo(page, at);
    page.scene
        .draw_rect(DrawRect::new(bounds, page.palette.disc).radius(DISC_RADIUS));
}

fn halo(page: &mut Page<'_>, at: Point) {
    let (width, height) = (page.rect.width, page.rect.height);
    let centre = (at.x - page.rect.origin.x, at.y - page.rect.origin.y);
    let glow = blend(page.palette.disc, page.palette.glow, page.warmth);
    let stops = HALO.map(|(offset, alpha)| (offset, glow.alpha(alpha)));
    path(whole(width), whole(height))
        .fill_radial(centre, HALO_RADIUS, stops.to_vec())
        .move_to(0.0, 0.0)
        .line_to(width, 0.0)
        .line_to(width, height)
        .line_to(0.0, height)
        .close()
        .draw(page.scene, page.rect, page.theme);
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a canvas is a small positive number of pixels on each side"
)]
fn whole(pixels: f32) -> u32 {
    pixels.round().max(1.0) as u32
}

struct Ridge {
    haze: f32,
    crest: [f32; 7],
}

const RIDGES: [Ridge; 3] = [
    Ridge {
        haze: 0.62,
        crest: [4.0, 1.0, 5.0, 2.0, 0.5, 3.5, 2.0],
    },
    Ridge {
        haze: 0.34,
        crest: [9.0, 6.0, 10.5, 7.0, 5.5, 9.0, 7.5],
    },
    Ridge {
        haze: 0.0,
        crest: [15.0, 12.5, 16.0, 11.5, 14.0, 17.0, 13.0],
    },
];

fn ridges(page: &mut Page<'_>) {
    let [_, cool, _] = page.palette.pigments;
    let mist = blend(cool, page.palette.glow, page.warmth * 0.7);
    for ridge in &RIDGES {
        let body = blend(page.palette.land, mist, ridge.haze);
        let crest = blend(body, mist, 0.35);
        ridge_path(ridge, crest, body).draw(page.scene, page.rect, page.theme);
    }
}

fn ridge_path(ridge: &Ridge, crest: Color, body: Color) -> PathBuilder {
    let [first, rest @ ..] = ridge.crest;
    let span = UNITS / 6.0;
    let mut line = path(100, 100)
        .fill_linear(
            (0.0, HORIZON),
            (0.0, HORIZON + 24.0),
            vec![(0.0, crest), (1.0, body)],
        )
        .move_to(0.0, UNITS)
        .line_to(0.0, HORIZON + first);
    let (mut x, mut y) = (0.0, HORIZON + first);
    for lift in rest {
        let (next_x, next_y) = (x + span, HORIZON + lift);
        let half = span / 2.0;
        line = line.cubic_to(x + half, y, next_x - half, next_y, next_x, next_y);
        (x, y) = (next_x, next_y);
    }
    line.line_to(UNITS, UNITS).close()
}
