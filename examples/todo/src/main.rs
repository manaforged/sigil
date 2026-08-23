//! `todo` — a to-do app that dogfoods Sigil chrome.
//!
//! - **GPU menu bar:** File and Edit, styled with `.class`. Native
//!   `set_menu_bar` is opt-in OS chrome; this example does not use it.
//! - **File dialogs:** Open… and Save… persist the list as a text file.
//! - **System tray:** an icon with a tooltip and a Quit menu.
//!
//! File → Documentation… calls `sigil::open`. Pure logic is unit-tested
//! (see the tests below and the `dialog::mock` seam).

mod state;
mod view;

use sigil::WindowCommand;
use sigil::dialog::{self, FileDialog};
use sigil::menu::{Menu, PredefinedMenuItem};
use sigil::prelude::*;
use sigil::tray::{self, Icon, TrayIconBuilder};

use crate::state::TodoState;
use crate::view::app_tree;

fn main() -> sigil::AppResult<()> {
    if let Some(builder) = build_tray() {
        tray::set_tray(builder);
    }
    sigil::launch(("Sigil \u{2014} Todo", 720, 560), build_root)
}

fn build_root(cx: &mut Cx) -> NodeId {
    let state = TodoState::seed();
    cx.add(app_tree(&state))
}

/// File / Edit actions from the GPU menu bar.
pub(crate) fn on_menu(state: TodoState, id: &str) {
    match id {
        "new" => state.new_list(),
        "open" => {
            if let Some(path) = FileDialog::new().filter("Text", &["txt"]).open_file() {
                state.load_from(&path);
            }
        }
        "save" => {
            if let Some(path) = FileDialog::new()
                .filter("Text", &["txt"])
                .file_name("todos.txt")
                .save_file()
            {
                state.save_to(&path);
            }
        }
        "clear" => {
            if dialog::confirm("Clear completed?", "Remove every completed task.") {
                state.clear_completed();
            }
        }
        "docs" => {
            if let Err(err) = sigil::open("https://github.com/ThomGardiner/sigil") {
                state.status.set(format!("Open failed: {err}"));
            }
        }
        "quit" => request_window_command(WindowCommand::Close),
        _ => {}
    }
}

/// A tray icon with a tooltip and a Quit context menu.
fn build_tray() -> Option<TrayIconBuilder> {
    let icon = Icon::from_rgba(accent_icon(), ICON_SIZE, ICON_SIZE).ok()?;
    let quit = PredefinedMenuItem::quit(None);
    let tray_menu = Menu::new();
    tray_menu.append(&quit).ok()?;
    Some(
        TrayIconBuilder::new()
            .with_tooltip("Sigil Todo")
            .with_icon(icon)
            .with_menu(Box::new(tray_menu)),
    )
}

const ICON_SIZE: u32 = 32;

/// A solid accent-coloured square for the tray icon.
fn accent_icon() -> Vec<u8> {
    let pixels = (ICON_SIZE * ICON_SIZE) as usize;
    let mut rgba = Vec::with_capacity(pixels * 4);
    for _ in 0..pixels {
        rgba.extend_from_slice(&[80, 120, 255, 255]);
    }
    rgba
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
