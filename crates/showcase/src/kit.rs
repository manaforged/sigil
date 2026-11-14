use sigil::prelude::*;

use crate::focus::{BLUR, glass, stage, theme};
use crate::overlay;

const CARD_WIDTH: f32 = 380.0;
const DRIFT: f32 = 40.0;
const LINES: usize = 4;
const NAMES: [&str; 6] = [
    "Backdrop blur",
    "Spring fling",
    "Nested 3D",
    "Enter and exit",
    "Gradient angle",
    "In view",
];
const SAVED: [&str; 3] = ["Draft saved", "Export finished", "Link copied"];

/// A marquee, a loading card, a toast stack and a closed bottom sheet over stage lights.
#[must_use]
pub fn app() -> impl IntoNode {
    sheet(false)
}

/// The same screen with the bottom sheet open when `open` is true.
#[must_use]
pub fn sheet(open: bool) -> impl IntoNode {
    let open = signal(open);
    let queue = ToastQueue::new();
    for message in SAVED {
        queue.success(message);
    }
    container()
        .theme(theme)
        .width(Length::Fill)
        .height(Length::Fill)
        .child(canvas(stage).width(Length::Fill).height(Length::Fill))
        .child(
            column()
                .layout(overlay())
                .padding(Space::Xxl)
                .gap(Space::Xxl)
                .align_items(AlignItems::Center)
                .child(marquee(names).speed(DRIFT).width(Length::Fill))
                .child(
                    row()
                        .gap(Space::Xl)
                        .child(loading())
                        .child(actions(open, queue)),
                ),
        )
        .child(toast_queue(queue))
        .child(filters(open))
}

fn names() -> NodeBuilder {
    row()
        .gap(Space::Xxl)
        .padding_x(Space::Lg)
        .children(NAMES.map(|name| text(name).text_role(TextRole::Heading)))
        .into()
}

fn panel(title: &'static str) -> NodeBuilder {
    let panel = column().width(CARD_WIDTH).padding(Space::Xl).gap(Space::Lg);
    glass(panel, BLUR).child(text(title).text_role(TextRole::Title))
}

fn loading() -> NodeBuilder {
    panel("Loading the report").child(skeleton().lines(LINES).shimmer(true))
}

fn actions(open: Signal<bool>, queue: ToastQueue) -> NodeBuilder {
    panel("Try it").child(
        row()
            .gap(Space::Md)
            .child(button("Filters").activate(move || open.set(true)))
            .child(
                button("Save")
                    .primary()
                    .activate(move || queue.success("Draft saved")),
            ),
    )
}

fn filters(open: Signal<bool>) -> impl IntoNode {
    let (unread, mine) = (signal(true), signal(false));
    modal(open)
        .side(Side::Bottom)
        .drag(true)
        .title("Filters")
        .child(
            column()
                .gap(Space::Md)
                .child(toggle(unread).label("Unread only"))
                .child(toggle(mine).label("Assigned to me")),
        )
}
