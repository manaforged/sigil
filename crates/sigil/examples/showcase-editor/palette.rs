use sigil::prelude::*;

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

pub(crate) fn commands(ws: Workspace) -> NodeBuilder {
    COMMANDS
        .iter()
        .fold(command_menu(ws.palette_open), |menu, (label, command)| {
            let command = *command;
            let title = command
                .hint()
                .map_or_else(|| String::from(*label), |hint| format!("{label}    {hint}"));
            menu.item(MenuItem::new(title).activate(move || run(ws, command)))
        })
        .placeholder("Type a command")
        .width(PALETTE_W)
        .into()
}

pub(crate) fn quick_open(ws: Workspace) -> NodeBuilder {
    FileId::ALL
        .into_iter()
        .fold(command_menu(ws.quick_open), |menu, file| {
            menu.item(MenuItem::new(file.path()).activate(move || ws.open_file(file)))
        })
        .placeholder("Search files by name")
        .width(PALETTE_W)
        .into()
}
