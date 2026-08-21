//! `todo` — a to-do app that dogfoods Sigil's native-OS integrations.
//!
//! Beyond the reactive UI (add / toggle / delete / filter tasks), this is
//! the on-device smoke test for `sigil::menu`, `sigil::dialog`, and
//! `sigil::tray`:
//!
//! - **Menu bar:** File (New / Open… / Save… / Quit), Edit (Clear Completed).
//! - **File dialogs:** Open… and Save… persist the list as a text file.
//! - **System tray:** an icon with a tooltip and a Quit menu.
//!
//! Run it on macOS or Windows: the menu bar and tray should appear, Open…
//! and Save… should open native pickers, and menu clicks should drive the
//! list. Those native behaviours are what can only be checked here, on a
//! real desktop — the pure logic is unit-tested (see the tests below and
//! the `dialog::mock` seam).

mod state;
mod view;

use sigil::dialog::FileDialog;
use sigil::menu::{self, Menu, MenuItem, PredefinedMenuItem, Submenu};
use sigil::prelude::*;
use sigil::tray::{self, Icon, TrayIconBuilder};

use crate::state::TodoState;
use crate::view::app_tree;

fn main() -> sigil::AppResult<()> {
    // Register the native menu bar + tray BEFORE launch; Sigil installs them
    // on the first frame.
    if let Some(bar) = build_menu_bar() {
        menu::set_menu_bar(bar);
    }
    if let Some(builder) = build_tray() {
        tray::set_tray(builder);
    }
    sigil::launch(("Sigil \u{2014} Todo", 720, 560), build_root)
}

fn build_root(cx: &mut Cx) -> NodeId {
    let state = TodoState::seed();
    wire_menu(state);
    cx.add(app_tree(&state))
}

/// File + Edit menus. Item ids are matched in [`wire_menu`].
fn build_menu_bar() -> Option<Menu> {
    let new = MenuItem::with_id("new", "New List", true, None);
    let open = MenuItem::with_id("open", "Open\u{2026}", true, None);
    let save = MenuItem::with_id("save", "Save\u{2026}", true, None);
    let separator = PredefinedMenuItem::separator();
    let quit = PredefinedMenuItem::quit(None);
    let file = Submenu::with_items("File", true, &[&new, &open, &save, &separator, &quit]).ok()?;

    let clear = MenuItem::with_id("clear", "Clear Completed", true, None);
    let edit = Submenu::with_items("Edit", true, &[&clear]).ok()?;

    let bar = Menu::new();
    bar.append(&file).ok()?;
    bar.append(&edit).ok()?;
    Some(bar)
}

/// Route menu-item clicks to state actions. Open/Save open native pickers.
fn wire_menu(state: TodoState) {
    let subscription = menu::events().subscribe(move |id| {
        if id == "new" {
            state.new_list();
        } else if id == "open" {
            if let Some(path) = FileDialog::new().filter("Text", &["txt"]).open_file() {
                state.load_from(&path);
            }
        } else if id == "save" {
            if let Some(path) = FileDialog::new()
                .filter("Text", &["txt"])
                .file_name("todos.txt")
                .save_file()
            {
                state.save_to(&path);
            }
        } else if id == "clear" {
            state.clear_completed();
        }
    });
    // App-lifetime subscription: keep it registered for the whole run.
    std::mem::forget(subscription);
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
