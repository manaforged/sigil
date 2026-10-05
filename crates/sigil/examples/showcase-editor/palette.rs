use sigil::prelude::*;
use sigil::widgets::CommandMenuBuilder;

use super::chrome::{Command, run};
use super::project::FileId;
use super::state::Workspace;

const PALETTE_W: f32 = 520.0;

const COMMANDS: &[(&str, Command)] = &[
    ("Run Tests", Command::RunTests),
    ("Save File", Command::Save),
    ("Close Editor", Command::CloseEditor),
    ("View: Toggle Outline", Command::ToggleOutline),
    ("View: Show Problems", Command::Problems),
    ("View: Show Terminal", Command::Terminal),
    ("Go to File...", Command::OpenFile),
];

fn file_items(ws: Workspace, open: Signal<bool>, menu: CommandMenuBuilder) -> CommandMenuBuilder {
    FileId::ALL.into_iter().fold(menu, |menu, file| {
        menu.item(file.path(), move || {
            open.set(false);
            ws.open_file(file);
        })
    })
}

pub(crate) fn commands(ws: Workspace) -> NodeBuilder {
    let open = ws.palette_open;
    let menu = COMMANDS
        .iter()
        .fold(command_menu(), |menu, (label, command)| {
            let command = *command;
            menu.item(*label, move || {
                open.set(false);
                run(ws, command);
            })
        })
        .placeholder("Type a command")
        .cancel(move || open.set(false))
        .width(PALETTE_W);
    container().child(modal(open).title("Command palette").child(menu))
}

pub(crate) fn quick_open(ws: Workspace) -> NodeBuilder {
    let open = ws.quick_open;
    let menu = file_items(ws, open, command_menu())
        .placeholder("Search files by name")
        .cancel(move || open.set(false))
        .width(PALETTE_W);
    container().child(modal(open).title("Go to file").child(menu))
}
