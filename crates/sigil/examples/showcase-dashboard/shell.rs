use sigil::prelude::{
    AlignItems, ControlSize, LayoutBuilderExt, LayoutStyle, NodeBuilder, Paint, Token, Tone,
    avatar, button, chip, command_menu, container, modal, spacer, status_bar, text, tooltip,
};
use sigil::style::space;
use sigil::widgets::ModalBuilder;

use super::data::{Bucket, Section, TEXT_SMALL};
use super::live::Console;

pub(crate) fn top_bar(console: Console) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(space(3))
                .align_items(AlignItems::Center),
        )
        .child(
            text(move || {
                let section = console.section.get().label();
                let service = console.service.get().name();
                format!("{section}  /  {service}")
            })
            .nowrap()
            .ellipsize()
            .text_color(Token::OnSurfaceMuted),
        )
        .child(
            chip(
                move || {
                    if console.paused.get() {
                        "Deploys paused".to_owned()
                    } else {
                        "Deploys open".to_owned()
                    }
                },
                Tone::Success,
            )
            .tone(move || {
                if console.paused.get() {
                    Tone::Warn
                } else {
                    Tone::Success
                }
            }),
        )
        .child(spacer())
        .child(
            tooltip("Search services, jobs, and commands")
                .shortcut(Some("Ctrl+K"))
                .child(
                    button("Search or run a command")
                        .ghost()
                        .size(ControlSize::Compact)
                        .activate(move || console.menu_open.set(true)),
                ),
        )
        .child(avatar("AN"))
}

pub(crate) fn status(console: Console) -> NodeBuilder {
    status_bar([
        text(move || console.link_text()).nowrap(),
        text(move || {
            let running = console.tally(None, Bucket::Running);
            let queued = console.tally(None, Bucket::Queued);
            let failed = console.tally(None, Bucket::Failed);
            format!("{running} running · {queued} queued · {failed} failed")
        })
        .nowrap()
        .tabular_nums(true),
        text(move || format!("Last event {} UTC", console.clock.get()))
            .nowrap()
            .tabular_nums(true),
        text("Ctrl+K Command menu   Space Select   Esc Close")
            .nowrap()
            .font_size(TEXT_SMALL)
            .text_color(Token::OnSurfaceMuted),
    ])
}

pub(crate) fn command_modal(console: Console) -> ModalBuilder {
    let open = console.menu_open;
    let menu = command_menu()
        .placeholder("Type a command")
        .item("Deploy the selected service", move || {
            open.set(false);
            console.deploy();
        })
        .item("Pause all deploys", move || {
            open.set(false);
            console.set_paused(true);
        })
        .item("Resume all deploys", move || {
            open.set(false);
            console.set_paused(false);
        })
        .item("Retry the selected job", move || {
            open.set(false);
            console.retry(console.focus.peek());
        })
        .submenu("Go to", move |nested| {
            Section::ALL.iter().fold(nested, |entries, &section| {
                entries.item(section.label(), move || console.go(section))
            })
        })
        .cancel(move || open.set(false));
    modal(open)
        .title("Command menu")
        .dismiss_on_backdrop(true)
        .child(menu)
}
