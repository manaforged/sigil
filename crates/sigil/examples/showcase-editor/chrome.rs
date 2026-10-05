use sigil::prelude::*;
use sigil::style::space;
use sigil::widgets::DiagSeverity;

use super::facts::{count, severity_icon, severity_tone};
use super::project::{AHEAD, BEHIND, BRANCH, PROJECT_NAME};
use super::state::{BottomTab, Workspace};

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

impl Command {
    pub(crate) const BOUND: [Self; 3] = [Self::OpenFile, Self::Save, Self::Palette];

    const fn key(self) -> Option<&'static str> {
        match self {
            Self::OpenFile => Some("p"),
            Self::Save => Some("s"),
            Self::Palette => Some("k"),
            Self::CloseEditor
            | Self::ToggleOutline
            | Self::Problems
            | Self::Terminal
            | Self::RunTests
            | Self::Shortcuts => None,
        }
    }

    pub(crate) fn shortcut(self) -> Option<Shortcut> {
        self.key()
            .map(|key| Shortcut::primary(LogicalKey::character(key)))
    }

    pub(crate) fn hint(self) -> Option<String> {
        self.shortcut().map(|shortcut| shortcut.label())
    }
}

type MenuEntry = (&'static str, Command);

const MENUS: &[(&str, &[MenuEntry])] = &[
    (
        "File",
        &[
            ("Open File...", Command::OpenFile),
            ("Save", Command::Save),
            ("Close Editor", Command::CloseEditor),
        ],
    ),
    ("Edit", &[("Command Palette...", Command::Palette)]),
    (
        "View",
        &[
            ("Toggle Outline", Command::ToggleOutline),
            ("Problems", Command::Problems),
            ("Terminal", Command::Terminal),
        ],
    ),
    ("Go", &[("Go to File...", Command::OpenFile)]),
    ("Run", &[("Run Tests", Command::RunTests)]),
    ("Help", &[("Keyboard Shortcuts", Command::Shortcuts)]),
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
    MENUS.iter().fold(menu_bar(), |bar, (title, entries)| {
        bar.menu(
            *title,
            entries.iter().map(|(label, command)| {
                let command = *command;
                let base = MenuItem::new(*label).activate(move || run(ws, command));
                command.hint().into_iter().fold(base, MenuItem::shortcut)
            }),
        )
    })
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
            button("Run tests")
                .icon(Icon::Play)
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
            icon(severity_icon(severity))
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
