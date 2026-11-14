use std::f32::consts::TAU;

use sigil::animation::{Animator, Spring, clock};
use sigil::prelude::*;
use sigil::scene::SceneBuilder;
use sigil::style::TextStyle;

use crate::sheet::{INK, PAPER, SIGNAL, label, mix, theme};

const WORD: [&str; 5] = ["s", "i", "g", "i", "l"];
const SIZE: f32 = 28.0;
const PITCH: [f32; 2] = [24.0, 34.0];
const SHIFT: usize = 2;
const REACH: f32 = 210.0;
const CORE: f32 = 1.4;
const THIN: f32 = 300.0;
const HEAVY: f32 = 800.0;
const STEP: f32 = 25.0;
const FAINT: f32 = 0.22;
const ORBIT: [f32; 2] = [0.11, 0.17];
const SWAY: [f32; 2] = [0.34, 0.28];
const INSET: f32 = 24.0;

#[derive(Clone, Copy)]
struct Probe {
    x: Animator,
    y: Animator,
    held: Animator,
    size: Signal<Size>,
}

impl Probe {
    fn new() -> Self {
        Self {
            x: Animator::spring(0.0, Spring::SMOOTH),
            y: Animator::spring(0.0, Spring::SMOOTH),
            held: Animator::spring(0.0, Spring::GENTLE),
            size: signal(Size::ZERO),
        }
    }

    fn follow(self, to: Point) {
        self.x.target(to.x);
        self.y.target(to.y);
        self.held.target(1.0);
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "seconds since the app started are far inside f32"
    )]
    fn centre(self, seconds: f64) -> Point {
        let time = seconds as f32;
        let size = self.size.get();
        let drift = Point::new(
            (0.5 + SWAY[0] * (time * ORBIT[0] * TAU).sin()) * size.width,
            (0.5 + SWAY[1] * (time * ORBIT[1] * TAU).cos()) * size.height,
        );
        let held = self.held.get();
        Point::new(
            (self.x.get() - drift.x).mul_add(held, drift.x),
            (self.y.get() - drift.y).mul_add(held, drift.y),
        )
    }
}

/// A field of letters whose weight and colour follow the pointer.
#[must_use]
pub fn app() -> impl IntoNode {
    let probe = Probe::new();
    let seconds = clock();
    let at = memo(move || probe.centre(seconds.get()));
    let field: NodeBuilder = canvas(move |scene, rect, _| paint(scene, rect, at.peek()))
        .redraw_on(at)
        .track_size(probe.size)
        .width(Length::Fill)
        .grow(1.0)
        .accessible("The word sigil repeated; letters near the pointer are heavier")
        .into();
    column()
        .theme(theme())
        .background(PAPER)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Space::Lg)
        .gap(Space::Md)
        .child(
            field
                .pointer_move(move |event| probe.follow(event.position()))
                .pointer_leave(move |_| probe.held.target(0.0)),
        )
        .child(readout(probe, at))
}

fn readout(probe: Probe, at: Signal<Point>) -> impl IntoNode {
    row()
        .width(Length::Fill)
        .justify_content(JustifyContent::SpaceBetween)
        .padding_top(Space::Sm)
        .border_top(1)
        .border_color(INK)
        .child(label(move || {
            let at = at.get();
            format!("X\u{a0}{:04.0}\u{a0}\u{a0}\u{a0}Y\u{a0}{:04.0}", at.x, at.y)
        }))
        .child(label(move || {
            let [columns, rows] = grid(probe.size.get());
            format!("GLYPHS\u{a0}{}", columns * rows)
        }))
        .child(label("WGHT 300 TO 800, ONE FONT FILE"))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "cell counts and font weights are small positive numbers"
)]
fn grid(size: Size) -> [usize; 2] {
    [
        ((size.width - 2.0 * INSET) / PITCH[0]).floor().max(0.0) as usize,
        ((size.height - 2.0 * INSET) / PITCH[1]).floor().max(0.0) as usize,
    ]
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "cell counts and font weights are small positive numbers"
)]
fn paint(scene: &mut SceneBuilder, rect: Rect, centre: Point) {
    let [columns, rows] = grid(Size::new(rect.width, rect.height));
    let left = (rect.width - columns as f32 * PITCH[0]) / 2.0;
    let top = (rect.height - rows as f32 * PITCH[1]) / 2.0;
    let plain = TextStyle::default().font_size(SIZE);
    let widths = WORD.map(|letter| scene.measure_text(letter, &plain).width);
    for row in 0..rows {
        for column in 0..columns {
            let letter = (column + row * SHIFT) % WORD.len();
            let x = (column as f32 + 0.5).mul_add(PITCH[0], left);
            let y = (row as f32).mul_add(PITCH[1], top);
            let away = (x - centre.x).hypot(y + PITCH[1] / 2.0 - centre.y) / REACH;
            let near = (-away * away).exp();
            let weight = ((HEAVY - THIN).mul_add(near, THIN) / STEP).round() * STEP;
            let style = plain.clone().font_weight(weight as u16);
            let core = near.powf(CORE);
            let ink = mix(INK.alpha((FAINT * 255.0) as u8), INK, (near * 3.0).min(1.0));
            let colour = mix(ink, SIGNAL, core);
            let origin = Point::new(rect.origin.x + x - widths[letter] / 2.0, rect.origin.y + y);
            scene.draw_text(origin, WORD[letter], &style, colour);
        }
    }
}
