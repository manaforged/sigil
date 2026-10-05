#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

mod chrome;
mod editor;
mod facts;
mod outline;
mod palette;
mod panel;
mod project;
mod state;
mod tree;

use sigil::prelude::*;
use sigil::style::space;

use state::Workspace;

const WINDOW_W: u32 = 1280;
const WINDOW_H: u32 = 800;
const TEXT_XS: f32 = 12.0;
const TEXT_SM: f32 = 14.0;
const WEIGHT_SEMIBOLD: u16 = 600;
const EXPLORER_W: f32 = 248.0;
const EXPLORER_MIN: f32 = 180.0;
const OUTLINE_W: f32 = 220.0;
const OUTLINE_MIN: f32 = 160.0;
const CENTER_MIN: f32 = 360.0;
const BOTTOM_H: f32 = 210.0;
const BOTTOM_MIN: f32 = 120.0;
const EDITOR_MIN: f32 = 200.0;

fn main() -> sigil::AppResult<()> {
    sigil::run(
        AppConfig::new("orbit - Sigil editor").size(WINDOW_W, WINDOW_H),
        || {
            set_theme(Theme::dark());
            app()
        },
    )
}

pub(crate) fn app() -> impl IntoNode {
    let ws = Workspace::new();
    container()
        .layout(LayoutStyle::flex_column().fill())
        .key_capture(Shortcut::primary(LogicalKey::character("k")), move |_| {
            ws.palette_open.set(true);
        })
        .key_capture(Shortcut::primary(LogicalKey::character("p")), move |_| {
            ws.quick_open.set(true);
        })
        .key_capture(Shortcut::primary(LogicalKey::character("s")), move |_| {
            ws.save_active();
        })
        .child(app_shell(
            chrome::top_bar(ws),
            workbench(ws),
            chrome::status(ws),
            None::<NodeBuilder>,
        ))
        .child(palette::commands(ws))
        .child(palette::quick_open(ws))
        .child(toast_queue(ws.toasts))
}

fn workbench(ws: Workspace) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill()
                .min_size(0.0, 0.0)
                .gap(space(2)),
        )
        .child(tree::activity_strip(ws))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .grow(1.0)
                        .fill_height()
                        .min_size(0.0, 0.0),
                )
                .children_reactive(move || [main_split(ws, ws.outline_open.get())]),
        )
}

fn main_split(ws: Workspace, with_outline: bool) -> NodeBuilder {
    let split = splitter(Orientation::Horizontal)
        .child_constrained(
            tree::side_bar(ws),
            EXPLORER_W,
            EXPLORER_MIN,
            EXPLORER_W * 2.0,
        )
        .pane(center(ws))
        .min(CENTER_MIN)
        .grow(1.0)
        .commit();
    let split = if with_outline {
        split.child_constrained(
            outline::outline(ws),
            OUTLINE_W,
            OUTLINE_MIN,
            OUTLINE_W * 2.0,
        )
    } else {
        split
    };
    split.grow(1.0).into()
}

fn center(ws: Workspace) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill().min_size(0.0, 0.0))
        .child(
            splitter(Orientation::Vertical)
                .pane(editor::editor(ws))
                .min(EDITOR_MIN)
                .grow(1.0)
                .commit()
                .child_constrained(
                    panel::bottom_panel(ws),
                    BOTTOM_H,
                    BOTTOM_MIN,
                    BOTTOM_H * 2.0,
                )
                .grow(1.0),
        )
}
