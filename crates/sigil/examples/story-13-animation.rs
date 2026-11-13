use std::time::Duration;

use sigil::animation::{Animator, Easing, Property, Spring, Transition, every_frame};
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run("Animation & Springs", app)
}

const TRACK_WIDTH: f32 = 160.0;
const BAR_HEIGHT: f32 = 16.0;

#[derive(Clone, Copy)]
struct AnimationDemo {
    drawer: Animator,
    fade: Animator,
    frames: Signal<u32>,
    elapsed: Signal<f32>,
    ticker_on: Signal<bool>,
}

/// Interactive Layer 2 screen exercising `Animator`, `Spring`, `Property::Opacity`, and `every_frame`.
#[must_use]
pub fn app() -> impl IntoNode {
    let demo = AnimationDemo {
        drawer: Animator::spring(0.0, Spring::SNAPPY),
        fade: Animator::tween(
            1.0,
            Transition::new()
                .duration(Duration::from_millis(240))
                .ease(Easing::CubicOut),
        ),
        frames: signal(0_u32),
        elapsed: signal(0.0_f32),
        ticker_on: signal(true),
    };

    column()
        .padding(Space::Md)
        .gap(Space::Sm)
        .background(Token::Surface)
        .child(text("Animation & Springs").text_role(TextRole::Title))
        .child(controls_row(demo))
        .child(drawer_preview(demo))
        .child(fade_card(demo))
        .child(status_readout(demo))
        .child(live_ticker(demo))
}

fn controls_row(demo: AnimationDemo) -> NodeBuilder {
    row()
        .gap(Space::Sm)
        .align_items(AlignItems::Center)
        .child(
            button("Toggle")
                .size(ControlSize::Compact)
                .activate(move || {
                    let next = if demo.drawer.value().peek() < 0.5 {
                        1.0
                    } else {
                        0.0
                    };
                    demo.drawer.target(next);
                    demo.fade.target(next);
                }),
        )
        .child(button("Snap").size(ControlSize::Compact).activate(move || {
            demo.drawer.snap(1.0);
            demo.fade.snap(1.0);
        }))
        .child(
            button("Cancel")
                .size(ControlSize::Compact)
                .activate(move || {
                    demo.drawer.cancel();
                    demo.fade.cancel();
                }),
        )
        .child(
            button("Ticker")
                .size(ControlSize::Compact)
                .activate(move || demo.ticker_on.update(|on| *on = !*on)),
        )
}

fn drawer_preview(demo: AnimationDemo) -> NodeBuilder {
    container()
        .width(TRACK_WIDTH)
        .height(BAR_HEIGHT)
        .radius(Radius::Sm)
        .background(Token::SurfaceSubtle)
        .children_reactive(move || {
            let width = demo.drawer.get().clamp(0.0, 1.0) * TRACK_WIDTH;
            [container()
                .width(Length::Px(width))
                .height(BAR_HEIGHT)
                .radius(Radius::Sm)
                .background(Token::Accent)]
        })
}

fn fade_card(demo: AnimationDemo) -> impl IntoNode {
    card(text("Faded panel").nowrap()).drive(Property::Opacity, demo.fade)
}

fn status_readout(demo: AnimationDemo) -> NodeBuilder {
    column()
        .gap(Space::Xs)
        .child(
            text(move || {
                let active = demo.drawer.active().get() || demo.fade.active().get();
                format!("Drawer: {:.2} | Active: {active}", demo.drawer.get())
            })
            .nowrap(),
        )
        .child(
            text(move || format!("Frames: {} ({:.3}s)", demo.frames.get(), demo.elapsed.get()))
                .nowrap(),
        )
}

fn live_ticker(demo: AnimationDemo) -> impl IntoNode {
    show(
        demo.ticker_on,
        move || {
            every_frame(move |dt| {
                demo.frames.update(|n| *n += 1);
                demo.elapsed.update(|s| *s += dt);
            });
            text("Ticker: on").nowrap()
        },
        || text("Ticker: off").nowrap(),
    )
}
