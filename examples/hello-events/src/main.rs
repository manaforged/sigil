//! Pointer-hit + focus demo on `sigil::launch`.
//!
//! Two side-by-side panes: clicking either one toggles the background
//! style. Tab cycles keyboard focus between the two panes.

#![allow(missing_docs)]

use sigil::prelude::*;
use sigil::reactive::signal;

static BG_OFF: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static BG_ON: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

fn main() -> sigil::Result<()> {
    // 720x480 logical pixels; physical size depends on monitor scale
    // (1080x720 at 1.5x, 1440x960 at 2x).
    sigil::launch(("Sigil - hello-events", 720, 480), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let toggled = signal(false);
    cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(720.0, 480.0))
            .style_reactive(move || if toggled.get() { BG_ON } else { BG_OFF })
            .child(hit_pane(toggled))
            .child(hit_pane(toggled)),
    )
}

fn hit_pane(toggled: sigil::reactive::Signal<bool>) -> NodeBuilder {
    container()
        .focusable(true)
        .layout(LayoutStyle::block().fixed_size(360.0, 480.0))
        .on_pointer_down(move |_| toggled.set(!toggled.peek()))
}
