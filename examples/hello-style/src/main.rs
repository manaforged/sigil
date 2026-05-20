//! Windowed style proof — static + hover-reactive panes via `sigil::launch`.

#![allow(missing_docs)]

use sigil::prelude::*;

static HOVER_OFF: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];
static HOVER_ON: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent-hover")];

fn main() -> sigil::Result<()> {
    // 720x480 logical pixels; physical size depends on monitor scale
    // (1080x720 at 1.5x, 1440x960 at 2x).
    assert!(!in_right_pane(720.0, 480.0));
    sigil::launch(("Sigil - hello-style", 720, 480), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let hover = signal(false);
    let pointer_hover = hover;
    cx.add(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(720.0, 480.0)
                    .gap(16.0),
            )
            .pointer_move_capture(move |event| {
                let inside = in_right_pane(event.position.x, event.position.y);
                if pointer_hover.peek() != inside {
                    pointer_hover.set(inside);
                }
            })
            .child(top_row(hover))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(688.0, 128.0))
                    .class("p-4 bg-accent rounded-md"),
            ),
    )
}

fn top_row(hover: sigil::reactive::Signal<bool>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().fixed_size(688.0, 304.0).gap(16.0))
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(328.0, 304.0))
                .class("p-8 bg-accent rounded-md"),
        )
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(328.0, 304.0))
                .class("p-8 rounded-md")
                .style_reactive(move || if hover.get() { HOVER_ON } else { HOVER_OFF }),
        )
}

fn in_right_pane(x: f32, y: f32) -> bool {
    (360.0..688.0).contains(&x) && (16.0..320.0).contains(&y)
}
