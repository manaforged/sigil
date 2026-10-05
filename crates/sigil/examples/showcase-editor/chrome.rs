use sigil::prelude::*;
use sigil::style::space;
use sigil::widgets::DiagSeverity;

use super::facts::{count, severity_glyph, severity_tone};
use super::project::{AHEAD, BEHIND, BRANCH, PROJECT_NAME};
use super::state::{BottomTab, Workspace};

const MENU_BAR_W: f32 = 380.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    OpenFile,
    Save,
    CloseEditor,
    Palette,
    ToggleOutline,
    Problems,
    Terminal,
    RunTests,
    Shortcuts,
}

struct Entry {
    label: &'static str,
    shortcut: Option<&'static str>,
    command: Command,
}

const fn entry(label: &'static str, shortcut: Option<&'static str>, command: Command) -> Entry {
    Entry {
        label,
        shortcut,
        command,
    }
}

const MENUS: &[(&str, &[Entry])] = &[
    (
        "File",
        &[
            entry("Open File...", Some("Ctrl+P"), Command::OpenFile),
            entry("Save", Some("Ctrl+S"), Command::Save),
            entry("Close Editor", None, Command::CloseEditor),
        ],
    ),
    (
        "Edit",
        &[entry(
            "Command Palette...",
            Some("Ctrl+K"),
            Command::Palette,
        )],
    ),
    (
        "View",
        &[
            entry("Toggle Outline", None, Command::ToggleOutline),
            entry("Problems", None, Command::Problems),
            entry("Terminal", None, Command::Terminal),
        ],
    ),
    (
        "Go",
        &[entry("Go to File...", Some("Ctrl+P"), Command::OpenFile)],
    ),
    ("Run", &[entry("Run Tests", None, Command::RunTests)]),
    (
        "Help",
        &[entry("Keyboard Shortcuts", None, Command::Shortcuts)],
    ),
];

pub(crate) fn run(ws: Workspace, command: Command) {
    match command {
        Command::OpenFile => ws.quick_open.set(true),
        Command::Save => ws.save_active(),
        Command::CloseEditor => ws.close_tab(ws.active_tab.get()),
        Command::Palette | Command::Shortcuts => ws.palette_open.set(true),
        Command::ToggleOutline => ws.outline_open.update(|open| *open = !*open),
        Command::Problems => ws.show_bottom(BottomTab::Problems),
        Command::Terminal => ws.show_bottom(BottomTab::Terminal),
        Command::RunTests => {
            ws.show_bottom(BottomTab::Terminal);
            ws.toasts.info("cargo test: 2 passed");
        }
    }
}

fn menus(ws: Workspace) -> sigil::widgets::MenuBarBuilder {
    MENUS
        .iter()
        .fold(menu_bar(), |bar, (label, entries)| {
            bar.menu(
                *label,
                entries.iter().map(|item| {
                    let command = item.command;
                    let base = MenuItem::new(item.label).activate(move || run(ws, command));
                    item.shortcut.into_iter().fold(base, MenuItem::shortcut)
                }),
            )
        })
        .width(MENU_BAR_W)
}

pub(crate) fn top_bar(ws: Workspace) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(space(2))
                .align_items(AlignItems::Center),
        )
        .child(menus(ws))
        .child(spacer())
        .child(
            text(PROJECT_NAME)
                .nowrap()
                .font_size(super::TEXT_SM)
                .font_weight(super::WEIGHT_SEMIBOLD),
        )
        .child(chip(BRANCH, Tone::Info))
        .child(spacer())
        .child(
            button("Build")
                .outline()
                .size(ControlSize::Compact)
                .activate(move || run(ws, Command::Problems)),
        )
        .child(
            button("▶ Run tests")
                .primary()
                .size(ControlSize::Compact)
                .activate(move || run(ws, Command::RunTests)),
        )
}

fn severity_item(severity: DiagSeverity) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(space(1))
                .align_items(AlignItems::Center),
        )
        .child(
            icon(severity_glyph(severity))
                .tone(severity_tone(severity))
                .size(ControlSize::Compact),
        )
        .child(
            text(count(severity).to_string())
                .nowrap()
                .font_size(super::TEXT_XS),
        )
}

fn caret_text(ws: Workspace) -> TextContent {
    TextContent::from(move || {
        ws.active().map_or_else(String::new, |file| {
            let (line, col) = ws.buffer(file).caret.get();
            format!("Ln {}, Col {}", line + 1, col + 1)
        })
    })
}

fn language_text(ws: Workspace) -> TextContent {
    TextContent::from(move || {
        ws.active()
            .map_or_else(String::new, |file| String::from(file.spec().kind.label()))
    })
}

fn right_group(ws: Workspace) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(space(4))
                .align_items(AlignItems::Center),
        )
        .child(text(caret_text(ws)).nowrap().font_size(super::TEXT_XS))
        .child(text("Spaces: 4").nowrap().font_size(super::TEXT_XS))
        .child(text("UTF-8").nowrap().font_size(super::TEXT_XS))
        .child(text("LF").nowrap().font_size(super::TEXT_XS))
        .child(text(language_text(ws)).nowrap().font_size(super::TEXT_XS))
        .child(
            button(TextContent::from(move || {
                String::from(if ws.outline_open.get() {
                    "Outline: on"
                } else {
                    "Outline: off"
                })
            }))
            .ghost()
            .size(ControlSize::Compact)
            .activate(move || run(ws, Command::ToggleOutline)),
        )
}

pub(crate) fn status(ws: Workspace) -> NodeBuilder {
    status_bar([
        container()
            .layout(
                LayoutStyle::flex_row()
                    .gap(space(1))
                    .align_items(AlignItems::Center),
            )
            .child(dot(Tone::Success))
            .child(text(BRANCH).nowrap().font_size(super::TEXT_XS)),
        container().child(
            text(format!("↑{AHEAD} ↓{BEHIND}"))
                .nowrap()
                .font_size(super::TEXT_XS),
        ),
        container()
            .layout(
                LayoutStyle::flex_row()
                    .gap(space(2))
                    .align_items(AlignItems::Center),
            )
            .pointer_down(move |_| run(ws, Command::Problems))
            .child(severity_item(DiagSeverity::Error))
            .child(severity_item(DiagSeverity::Warning))
            .child(severity_item(DiagSeverity::Info)),
        right_group(ws),
    ])
}
