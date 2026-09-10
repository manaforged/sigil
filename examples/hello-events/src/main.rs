#![allow(missing_docs)]

use sigil::prelude::*;

static BG_OFF: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static BG_ON: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

fn main() -> sigil::AppResult<()> {
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
        .pointer_down(move |_| toggled.set(!toggled.peek()))
}
