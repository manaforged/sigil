use std::f32::consts::{PI, TAU};

use sigil::animation::{Animator, Spring, clock};
use sigil::events::DragPhase;
use sigil::prelude::*;
use sigil::scene::{DrawRect, SceneBuilder, Shadow};

use crate::sheet::{INK, PAPER, SIGNAL, label, theme};

const CAP: Color = Color::rgb(30, 30, 32);
const CROWN: Color = Color::rgb(48, 48, 51);
const SCREEN: Color = Color::rgb(14, 17, 16);
const TRACE: Color = Color::rgb(226, 240, 228);
const HALO: Color = Color::rgba(226, 240, 228, 26);
const GRID: Color = Color::rgba(226, 240, 228, 14);
const DROP: [f32; 4] = [0.0, 0.0, 0.0, 0.34];
const LIFT: [f32; 2] = [0.0, 4.0];
const SOFT: f32 = 8.0;

const SCOPE: f32 = 608.0;
const BEZEL: f32 = 14.0;
const MARGIN: f32 = 36.0;
const DIVISIONS: u16 = 8;
const POINTS: u16 = 2000;
const TURNS: f32 = 16.0;
const HALO_WIDTH: f32 = 4.0;
const TRACE_WIDTH: f32 = 1.4;

const KNOB: f32 = 132.0;
const CAP_RADIUS: f32 = 44.0;
const CROWN_RADIUS: f32 = 36.0;
const TICK_RADIUS: f32 = 58.0;
const TICK: f32 = 6.0;
const TICKS: u16 = 11;
const POINTER: [f32; 2] = [14.0, 34.0];
const SWEEP_START: f32 = 0.75 * PI;
const SWEEP: f32 = 1.5 * PI;
const TRAVEL: f32 = 220.0;
const DRIFT: f32 = 0.12;
const READOUT: f32 = 22.0;
const WHY: &str = "an engraved readout outside the type scale of an app";

#[derive(Clone, Copy)]
struct Dial {
    name: &'static str,
    unit: &'static str,
    low: f32,
    high: f32,
    set: Signal<f32>,
    turn: Animator,
}

impl Dial {
    fn new(name: &'static str, unit: &'static str, [low, high]: [f32; 2], at: f32) -> Self {
        let set = signal(at);
        let turn = Animator::spring(at, Spring::SNAPPY);
        effect(move || turn.target(set.get()));
        Self {
            name,
            unit,
            low,
            high,
            set,
            turn,
        }
    }

    fn reading(self, turn: f32) -> f32 {
        (self.high - self.low).mul_add(turn, self.low)
    }
}

#[derive(Clone, Copy)]
struct Deck {
    across: Dial,
    up: Dial,
    phase: Dial,
    decay: Dial,
    running: Signal<bool>,
    drift: Signal<f32>,
}

impl Deck {
    fn new(running: bool) -> Self {
        let deck = Self {
            across: Dial::new("ACROSS", "HZ", [1.0, 5.0], 0.5),
            up: Dial::new("UP", "HZ", [1.0, 5.0], 0.255),
            phase: Dial::new("PHASE", "RAD", [0.0, PI], 0.5),
            decay: Dial::new("DECAY", "/S", [0.0, 0.05], 0.36),
            running: signal(running),
            drift: signal(0.0_f32),
        };
        deck.run();
        deck
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "a frame step in seconds is far inside f32"
    )]
    fn run(self) {
        let seconds = clock();
        let mut previous = None::<f64>;
        effect(move || {
            let now = seconds.get();
            let step = previous.replace(now).map_or(0.0, |before| now - before) as f32;
            if self.running.peek() {
                self.drift.update(|at| *at = DRIFT.mul_add(step, *at).rem_euclid(TAU));
            }
        });
    }

    fn dials(self) -> [Dial; 4] {
        [self.across, self.up, self.phase, self.decay]
    }

    fn turns(self) -> [f32; 5] {
        let [across, up, phase, decay] = self.dials().map(|dial| dial.turn.get());
        [across, up, phase, decay, self.drift.get()]
    }
}

/// The console with its trace drifting.
#[must_use]
pub fn app() -> impl IntoNode {
    board(Deck::new(true))
}

/// The console held still, for a screenshot.
#[must_use]
pub fn still() -> impl IntoNode {
    board(Deck::new(false))
}

fn board(deck: Deck) -> impl IntoNode {
    row()
        .theme(theme())
        .background(PAPER)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Space::Xxxl)
        .gap(Space::Xxxl)
        .align_items(AlignItems::Center)
        .child(scope(deck))
        .child(panel(deck))
}

fn scope(deck: Deck) -> impl IntoNode {
    canvas(move |scene, rect, theme| {
        let [across, up, phase, decay, drift] = deck.turns_now();
        let figure = Figure {
            across: deck.across.reading(across),
            up: deck.up.reading(up),
            phase: deck.phase.reading(phase) + drift,
            decay: deck.decay.reading(decay),
        };
        figure.paint(scene, rect, theme);
    })
    .redraw_on(move || deck.turns())
    .width(SCOPE)
    .height(SCOPE)
    .shrink(0.0)
    .accessible("Trace of the two oscillators")
}

impl Deck {
    fn turns_now(self) -> [f32; 5] {
        let [across, up, phase, decay] = self.dials().map(|dial| dial.turn.peek());
        [across, up, phase, decay, self.drift.peek()]
    }
}

struct Figure {
    across: f32,
    up: f32,
    phase: f32,
    decay: f32,
}

impl Figure {
    fn at(&self, time: f32) -> [f32; 2] {
        let fade = (-self.decay * time).exp();
        [
            fade * self.across.mul_add(time, self.phase).sin(),
            fade * (self.up * time).sin(),
        ]
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a canvas is a small positive number of pixels on each side"
    )]
    fn paint(&self, scene: &mut SceneBuilder, rect: Rect, theme: &Theme) {
        scene.draw_rect(DrawRect::new(rect, INK).radius(BEZEL));
        let glass = inset(rect, BEZEL);
        scene.draw_rect(DrawRect::new(glass, SCREEN).radius(BEZEL / 2.0));
        graticule(scene, glass);
        let field = inset(glass, MARGIN);
        let half = field.width / 2.0;
        let side = field.width.round().max(1.0) as u32;
        for (color, width) in [(HALO, HALO_WIDTH), (TRACE, TRACE_WIDTH)] {
            let line = (0..=POINTS).fold(path(side, side).stroke(color, width), |line, step| {
                let time = f32::from(step) / f32::from(POINTS) * TURNS * TAU;
                let [x, y] = self.at(time).map(|unit| unit.mul_add(half, half));
                if step == 0 {
                    line.move_to(x, y)
                } else {
                    line.line_to(x, y)
                }
            });
            line.draw(scene, field, theme);
        }
    }
}

fn inset(rect: Rect, by: f32) -> Rect {
    Rect::new(
        Point::new(rect.origin.x + by, rect.origin.y + by),
        2.0f32.mul_add(-by, rect.width),
        2.0f32.mul_add(-by, rect.height),
    )
}

fn graticule(scene: &mut SceneBuilder, glass: Rect) {
    let pitch = glass.width / f32::from(DIVISIONS);
    for line in 1..DIVISIONS {
        let offset = f32::from(line) * pitch;
        let (x, y) = (glass.origin.x + offset, glass.origin.y + offset);
        let (right, bottom) = (glass.origin.x + glass.width, glass.origin.y + glass.height);
        scene.stroke_line(Point::new(x, glass.origin.y), Point::new(x, bottom), GRID, 1.0);
        scene.stroke_line(Point::new(glass.origin.x, y), Point::new(right, y), GRID, 1.0);
    }
}

fn panel(deck: Deck) -> impl IntoNode {
    column()
        .height(SCOPE)
        .grow(1.0)
        .justify_content(JustifyContent::SpaceBetween)
        .child(plate())
        .child(
            column()
                .gap(Space::Xl)
                .child(pair(deck.across, deck.up))
                .child(pair(deck.phase, deck.decay)),
        )
        .child(transport(deck))
}

fn plate() -> impl IntoNode {
    row()
        .width(Length::Fill)
        .justify_content(JustifyContent::SpaceBetween)
        .padding_bottom(Space::Sm)
        .border_bottom(1)
        .border_color(INK)
        .child(label("SIGIL  /  HARMONOGRAPH"))
        .child(label("MODEL 02"))
}

fn pair(left: Dial, right: Dial) -> impl IntoNode {
    row()
        .width(Length::Fill)
        .justify_content(JustifyContent::SpaceAround)
        .child(knob(left))
        .child(knob(right))
}

fn knob(dial: Dial) -> impl IntoNode {
    let face: NodeBuilder = canvas(move |scene, rect, _| cap(scene, rect, dial.turn.peek()))
        .redraw_on(dial.turn.value())
        .width(KNOB)
        .height(KNOB)
        .into();
    column()
        .gap(Space::Xs)
        .align_items(AlignItems::Center)
        .child(
            face.accessible(dial.name)
                .pointer_drag(move |phase, event| {
                    if phase == DragPhase::Move {
                        let turn = dial.set.peek() - event.delta().y / TRAVEL;
                        dial.set.set(turn.clamp(0.0, 1.0));
                    }
                }),
        )
        .child(label(dial.name))
        .child(
            text(move || format!("{:.2} {}", dial.reading(dial.set.get()), dial.unit))
                .font_size(READOUT, WHY)
                .font_family(FontFamily::Mono)
                .tabular_nums(true)
                .text_color(INK),
        )
}

fn cap(scene: &mut SceneBuilder, rect: Rect, turn: f32) {
    let centre = Point::new(rect.origin.x + rect.width / 2.0, rect.origin.y + rect.height / 2.0);
    let along = |angle: f32, radius: f32| {
        Point::new(
            radius.mul_add(angle.cos(), centre.x),
            radius.mul_add(angle.sin(), centre.y),
        )
    };
    for tick in 0..TICKS {
        let angle = SWEEP.mul_add(f32::from(tick) / f32::from(TICKS - 1), SWEEP_START);
        scene.stroke_line(along(angle, TICK_RADIUS - TICK), along(angle, TICK_RADIUS), INK, 1.0);
    }
    let body = disc(centre, CAP_RADIUS);
    scene.draw_shadow(body, [CAP_RADIUS; 4], Shadow::new(LIFT, SOFT, 0.0, DROP));
    scene.draw_rect(DrawRect::new(body, CAP).radius(CAP_RADIUS));
    scene.draw_rect(DrawRect::new(disc(centre, CROWN_RADIUS), CROWN).radius(CROWN_RADIUS));
    let angle = SWEEP.mul_add(turn, SWEEP_START);
    scene.stroke_line(along(angle, POINTER[0]), along(angle, POINTER[1]), SIGNAL, 3.0);
}

fn disc(centre: Point, radius: f32) -> Rect {
    Rect::new(
        Point::new(centre.x - radius, centre.y - radius),
        radius * 2.0,
        radius * 2.0,
    )
}

fn transport(deck: Deck) -> impl IntoNode {
    row()
        .width(Length::Fill)
        .gap(Space::Md)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::SpaceBetween)
        .child(label("DRAG A KNOB UP OR DOWN"))
        .child(toggle(deck.running).label("Drift"))
}
