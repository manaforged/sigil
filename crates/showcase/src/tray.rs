use std::time::Duration;

use sigil::animation::{Animator, Property, Spring};
use sigil::events::DragPhase;
use sigil::prelude::*;

use crate::focus::{BLUR, glass, stage, theme};
use crate::overlay;

const WIDTH: f32 = 520.0;
const HEIGHT: f32 = 440.0;
const LOWERED: f32 = 320.0;
const RAISED: f32 = 0.0;
const STOPS: [f32; 2] = [RAISED, LOWERED];
const GRIP_WIDTH: f32 = 44.0;
const GRIP_HEIGHT: f32 = 5.0;
const SWAP: Duration = Duration::from_millis(260);
const RISE: f32 = 12.0;
const SLIDE: f32 = 48.0;
const GRIP: Color = Color::rgba(255, 255, 255, 150);

struct Session {
    start: &'static str,
    task: &'static str,
    minutes: u32,
}

const SESSIONS: [Session; 5] = [
    Session {
        start: "09:00",
        task: "Review the renderer diff",
        minutes: 50,
    },
    Session {
        start: "10:05",
        task: "Write the changelog",
        minutes: 25,
    },
    Session {
        start: "10:40",
        task: "Reply to issues",
        minutes: 25,
    },
    Session {
        start: "13:15",
        task: "Profile the layout pass",
        minutes: 50,
    },
    Session {
        start: "14:20",
        task: "Record the demo",
        minutes: 25,
    },
];

/// The session tray lowered, ready to be dragged up.
#[must_use]
pub fn app() -> impl IntoNode {
    lifted(0.0)
}

/// The session tray lifted by `share` of its travel, from 0 lowered to 1 raised.
#[must_use]
pub fn lifted(share: f32) -> impl IntoNode {
    listing(signal((0..SESSIONS.len()).collect()), share)
}

/// The session tray lifted by `share`, listing the sessions whose indexes are in `open`.
#[must_use]
pub fn listing(open: Signal<Vec<usize>>, share: f32) -> impl IntoNode {
    let drop = Animator::spring((RAISED - LOWERED).mul_add(share, LOWERED), Spring::SNAPPY);
    container()
        .theme(theme)
        .width(Length::Fill)
        .height(Length::Fill)
        .child(canvas(stage).width(Length::Fill).height(Length::Fill))
        .child(
            column()
                .layout(overlay())
                .justify_content(JustifyContent::End)
                .align_items(AlignItems::Center)
                .child(tray(drop, open)),
        )
}

fn tray(drop: Animator, open: Signal<Vec<usize>>) -> NodeBuilder {
    let tray = column()
        .width(WIDTH)
        .height(HEIGHT)
        .padding(Space::Xl)
        .gap(Space::Lg)
        .align_items(AlignItems::Center);
    glass(tray, BLUR)
        .drive(Property::TranslateY, drop.value())
        .pointer_drag(move |phase, event| match phase {
            DragPhase::Move => drop.snap((drop.peek() + event.delta().y).clamp(RAISED, LOWERED)),
            DragPhase::End => {
                let _ = drop.fling_to(event.velocity().y, &STOPS);
            }
            _ => {}
        })
        .child(
            container()
                .width(GRIP_WIDTH)
                .height(GRIP_HEIGHT)
                .radius(Radius::Full)
                .background(GRIP),
        )
        .child(
            row()
                .width(Length::Fill)
                .justify_content(JustifyContent::SpaceBetween)
                .child(text("Today").text_role(TextRole::Title))
                .child(
                    text(move || format!("{} to do", open.with(Vec::len)))
                        .text_role(TextRole::Body)
                        .tabular_nums(true),
                ),
        )
        .child(column().width(Length::Fill).gap(Space::Md).child(for_each(
            open,
            |index| *index,
            move |index| session(*index, open),
        )))
        .into()
}

fn session(index: usize, open: Signal<Vec<usize>>) -> impl IntoNode + use<> {
    let session = SESSIONS.get(index);
    let task = session.map_or("", |session| session.task);
    row()
        .width(Length::Fill)
        .gap(Space::Lg)
        .align_items(AlignItems::Center)
        .transition(
            Transition::new()
                .duration(SWAP)
                .ease(Easing::CubicOut)
                .layout(),
        )
        .enter(Property::Opacity(0.0))
        .enter(Property::TranslateY(RISE))
        .exit(Property::Opacity(0.0))
        .exit(Property::TranslateX(SLIDE))
        .child(
            text(session.map_or("", |session| session.start))
                .text_role(TextRole::Body)
                .tabular_nums(true),
        )
        .child(text(task).text_role(TextRole::Body).grow(1.0))
        .child(
            text(format!(
                "{} min",
                session.map_or(0, |session| session.minutes)
            ))
            .text_role(TextRole::Body)
            .tabular_nums(true),
        )
        .child(
            button("Done")
                .accessible(format!("Done: {task}"))
                .activate(move || open.update(|open| open.retain(|other| *other != index))),
        )
}
