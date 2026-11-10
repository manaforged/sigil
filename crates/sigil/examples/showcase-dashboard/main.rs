#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

mod data;
mod deploys;
mod inspector;
mod jobs;
mod live;
mod nav;
mod seed;
mod services;
mod shell;

use sigil::prelude::*;

use data::INSPECTOR_BREAKPOINT;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Fleet").size(1280, 800), || {
        set_theme(Theme::dark());
        app()
    })
}

fn pane() -> NodeBuilder {
    column()
        .gap(Space::None)
        .background(Token::SurfaceSubtle)
        .border_width(1)
        .border_color(Token::Border)
        .radius(Radius::Md)
}

fn work(console: live::Console) -> NodeBuilder {
    let state = deploys::job_state(console);
    let viewport = sigil::app::viewport_size();
    let roomy = move || {
        let width = viewport.get().width;
        width <= 0.0 || width >= INSPECTOR_BREAKPOINT
    };
    row()
        .fill_width()
        .grow(1.0)
        .min_width(0.0)
        .min_height(0.0)
        .gap(Space::Sm)
        .align_items(AlignItems::Stretch)
        .child(nav::rail(console))
        .child(services::panel(console))
        .child(jobs::main_panel(console, &state))
        .child(inspector::panel(console).visible_when(roomy))
}

pub(crate) fn app() -> impl IntoNode {
    let console = live::start();
    let open = console.menu_open;
    column()
        .fill()
        .gap(Space::None)
        .key_capture(shell::menu_shortcut(), move |_: &mut KeyEvent| {
            open.set(true);
        })
        .child(app_shell(
            shell::top_bar(console),
            work(console),
            shell::status(console),
            None::<NodeBuilder>,
        ))
        .child(shell::command_modal(console))
        .child(toast_queue(console.toasts))
}
