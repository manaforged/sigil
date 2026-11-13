use crate::app::Wake;
use crate::app::bridge::Native;
use crate::error::{AppResult, Error, ErrorKind};
use crate::events::{Key, Modifiers, NamedKey, Shortcut};
use crate::reactive::EventChannel;
use crate::shell::key_map::same_name_match;
use muda::accelerator::{Key as NativeKey, KeyAccelerator, Modifiers as NativeModifiers};
use muda::{CheckMenuItem, IsMenuItem, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

const NATIVE_MENU_BAR: bool = cfg!(vendor_os);
const UNSUPPORTED: &str = "native menu bars are unsupported on this platform";

thread_local! {
    static NATIVE: Native<Menu, muda::Menu, String> = Native::new();
}

/// A menu item the platform implements.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StandardItem {
    /// Shows the platform's About panel for the app.
    About,
    /// Opens the platform's Services submenu.
    Services,
    /// Hides the windows of this app.
    Hide,
    /// Hides the windows of every other app.
    HideOthers,
    /// Shows the windows of every hidden app again.
    ShowAll,
    /// Quits the app.
    Quit,
    /// Undoes the last edit in the focused control.
    Undo,
    /// Redoes the edit that the last undo removed.
    Redo,
    /// Moves the selection to the clipboard.
    Cut,
    /// Copies the selection to the clipboard.
    Copy,
    /// Inserts the clipboard at the caret.
    Paste,
    /// Selects everything in the focused control.
    SelectAll,
    /// Minimizes the focused window.
    Minimize,
    /// Maximizes the focused window.
    Maximize,
    /// Switches the focused window in or out of full screen.
    Fullscreen,
    /// Brings every window of this app in front of other apps' windows.
    BringAllToFront,
    /// Closes the focused window.
    CloseWindow,
}

#[derive(Clone, Debug)]
enum MenuEntry {
    Item {
        id: String,
        label: String,
        enabled: bool,
        shortcut: Option<Shortcut>,
    },
    Check {
        id: String,
        label: String,
        checked: bool,
        shortcut: Option<Shortcut>,
    },
    Standard(StandardItem),
    Separator,
    Submenu {
        label: String,
        menu: Menu,
    },
}

/// A native menu: the top level is the menu bar.
#[must_use]
#[derive(Clone, Debug, Default)]
pub struct Menu {
    entries: Vec<MenuEntry>,
}

impl Menu {
    /// An empty menu.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Adds an item; `events` delivers `id` when the user picks it.
    pub fn item(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        shortcut: Option<Shortcut>,
    ) -> Self {
        self.entries.push(MenuEntry::Item {
            id: id.into(),
            label: label.into(),
            enabled: true,
            shortcut,
        });
        self
    }

    /// Adds an item the user cannot pick.
    pub fn disabled(mut self, id: impl Into<String>, label: impl Into<String>) -> Self {
        self.entries.push(MenuEntry::Item {
            id: id.into(),
            label: label.into(),
            enabled: false,
            shortcut: None,
        });
        self
    }

    /// Adds an item with a check mark.
    pub fn check(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        checked: bool,
        shortcut: Option<Shortcut>,
    ) -> Self {
        self.entries.push(MenuEntry::Check {
            id: id.into(),
            label: label.into(),
            checked,
            shortcut,
        });
        self
    }

    /// Adds an item the platform implements.
    pub fn standard(mut self, item: StandardItem) -> Self {
        self.entries.push(MenuEntry::Standard(item));
        self
    }

    /// Adds a separator line.
    pub fn separator(mut self) -> Self {
        self.entries.push(MenuEntry::Separator);
        self
    }

    /// Adds a nested menu.
    pub fn submenu(mut self, label: impl Into<String>, menu: Self) -> Self {
        self.entries.push(MenuEntry::Submenu {
            label: label.into(),
            menu,
        });
        self
    }

    /// Queues the menu as the native menu bar for the next frame; an error means this platform has none.
    pub fn install(self) -> AppResult<()> {
        if !NATIVE_MENU_BAR {
            return Err(Error::fixed(ErrorKind::Other, UNSUPPORTED));
        }
        NATIVE.with(|native| native.queue(self));
        Ok(())
    }

    /// The id of each menu item the user picks.
    #[must_use]
    pub fn events() -> EventChannel<String> {
        NATIVE.with(Native::events)
    }

    pub(crate) fn native(&self) -> Result<muda::Menu, String> {
        let menu = muda::Menu::new();
        for entry in &self.entries {
            let item = native_entry(entry)?;
            menu.append(item.as_ref())
                .map_err(|error| error.to_string())?;
        }
        Ok(menu)
    }
}

fn native_entry(entry: &MenuEntry) -> Result<Box<dyn IsMenuItem>, String> {
    Ok(match entry {
        MenuEntry::Item {
            id,
            label,
            enabled,
            shortcut,
        } => {
            let item = MenuItem::with_id(id.as_str(), label, *enabled, None);
            warn_on_accelerator(item.set_key_accelerator(item_accelerator(shortcut.as_ref())));
            Box::new(item)
        }
        MenuEntry::Check {
            id,
            label,
            checked,
            shortcut,
        } => {
            let item = CheckMenuItem::with_id(id.as_str(), label, true, *checked, None);
            warn_on_accelerator(item.set_key_accelerator(item_accelerator(shortcut.as_ref())));
            Box::new(item)
        }
        MenuEntry::Standard(item) => Box::new(native_standard(*item)),
        MenuEntry::Separator => Box::new(PredefinedMenuItem::separator()),
        MenuEntry::Submenu { label, menu } => {
            let native = Submenu::new(label, true);
            for child in &menu.entries {
                let item = native_entry(child)?;
                native
                    .append(item.as_ref())
                    .map_err(|error| error.to_string())?;
            }
            Box::new(native)
        }
    })
}

fn item_accelerator(shortcut: Option<&Shortcut>) -> Option<KeyAccelerator> {
    accelerator(shortcut?)
        .inspect_err(|error| warn_on_accelerator(Err::<(), _>(error)))
        .ok()
}

fn warn_on_accelerator<E: core::fmt::Display>(result: Result<(), E>) {
    if let Err(error) = result {
        tracing::warn!(target: "sigil::menu", %error, "menu item has no accelerator");
    }
}

fn native_standard(item: StandardItem) -> PredefinedMenuItem {
    match item {
        StandardItem::About => PredefinedMenuItem::about(None, None),
        StandardItem::Services => PredefinedMenuItem::services(None),
        StandardItem::Hide => PredefinedMenuItem::hide(None),
        StandardItem::HideOthers => PredefinedMenuItem::hide_others(None),
        StandardItem::ShowAll => PredefinedMenuItem::show_all(None),
        StandardItem::Quit => PredefinedMenuItem::quit(None),
        StandardItem::Undo => PredefinedMenuItem::undo(None),
        StandardItem::Redo => PredefinedMenuItem::redo(None),
        StandardItem::Cut => PredefinedMenuItem::cut(None),
        StandardItem::Copy => PredefinedMenuItem::copy(None),
        StandardItem::Paste => PredefinedMenuItem::paste(None),
        StandardItem::SelectAll => PredefinedMenuItem::select_all(None),
        StandardItem::Minimize => PredefinedMenuItem::minimize(None),
        StandardItem::Maximize => PredefinedMenuItem::maximize(None),
        StandardItem::Fullscreen => PredefinedMenuItem::fullscreen(None),
        StandardItem::BringAllToFront => PredefinedMenuItem::bring_all_to_front(None),
        StandardItem::CloseWindow => PredefinedMenuItem::close_window(None),
    }
}

/// Delivers `id` as if the user picked that item.
#[cfg(feature = "kittest")]
pub fn simulate(id: impl Into<String>) {
    Menu::events().emit(id.into());
}

pub(crate) fn has_pending() -> bool {
    NATIVE.with(Native::has_pending)
}

pub(crate) fn install(hwnd: Option<isize>, wake: impl Fn() -> Wake) {
    if !has_pending() {
        return;
    }
    connect(wake);
    if let Err(error) = install_menu(hwnd) {
        tracing::error!("sigil-app: cannot install native menu: {error}");
    }
}

pub(crate) fn connect(wake: impl Fn() -> Wake) {
    let Some(sink) = NATIVE.with(|native| native.sink_once(&wake)) else {
        return;
    };
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| sink(event.id.0)));
}

pub(crate) fn pump() {
    NATIVE.with(Native::pump);
}

fn install_menu(hwnd: Option<isize>) -> Result<bool, String> {
    NATIVE.with(|native| {
        native.install(|menu| {
            let menu = menu.native()?;
            install_native(&menu, hwnd)?;
            Ok(menu)
        })
    })
}

pub(crate) fn accelerator(shortcut: &Shortcut) -> Result<KeyAccelerator, String> {
    let key = match &shortcut.key {
        Key::Named(key) => native_named_key(*key)
            .ok_or_else(|| format!("menu shortcut key {key} has no native key"))?,
        Key::Character(text) => NativeKey::Character(text.clone()),
    };
    Ok(KeyAccelerator::new(
        Some(native_modifiers(shortcut.modifiers)),
        key,
    ))
}

fn native_modifiers(modifiers: Modifiers) -> NativeModifiers {
    let pairs = [
        (Modifiers::SHIFT, NativeModifiers::SHIFT),
        (Modifiers::CTRL, NativeModifiers::CONTROL),
        (Modifiers::ALT, NativeModifiers::ALT),
        (Modifiers::META, NativeModifiers::SUPER),
    ];
    pairs
        .into_iter()
        .filter(|(ours, _)| modifiers.contains(*ours))
        .fold(NativeModifiers::empty(), |acc, (_, native)| acc | native)
}

const fn native_named_key(key: NamedKey) -> Option<NativeKey> {
    Some(
        same_name_match!(key, NamedKey => NativeKey, default: return None, [
            Enter, Tab, Escape, Backspace, Delete, Insert, Home, End, PageUp, PageDown,
            ArrowUp, ArrowDown, ArrowLeft, ArrowRight, PrintScreen, ScrollLock, Pause,
            CapsLock, NumLock, ContextMenu, Help, KanaMode, Convert, NonConvert, WakeUp,
            F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
            F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,
            BrowserBack, BrowserFavorites, BrowserForward, BrowserHome, BrowserRefresh,
            BrowserSearch, BrowserStop, AudioVolumeDown, AudioVolumeMute, AudioVolumeUp,
            MediaPlayPause, MediaStop, MediaTrackNext, MediaTrackPrevious, LaunchMail,
        ]),
    )
}

#[cfg(target_os = "macos")]
#[expect(
    clippy::unnecessary_wraps,
    reason = "all native menu adapters share one fallible interface"
)]
fn install_native(menu: &muda::Menu, _hwnd: Option<isize>) -> Result<(), String> {
    menu.init_for_nsapp();
    Ok(())
}

#[cfg(target_os = "windows")]
#[expect(
    unsafe_code,
    reason = "muda requires a raw live HWND to install the native menu"
)]
fn install_native(menu: &muda::Menu, hwnd: Option<isize>) -> Result<(), String> {
    let hwnd = hwnd.ok_or_else(|| "main window has no Win32 handle".to_string())?;
    // SAFETY: `hwnd` comes from the live winit window the shell still owns; `NATIVE` keeps `menu` alive after this call.
    unsafe { menu.init_for_hwnd(hwnd) }.map_err(|error| error.to_string())
}

#[cfg(not(vendor_os))]
fn install_native(_menu: &muda::Menu, _hwnd: Option<isize>) -> Result<(), String> {
    Err(UNSUPPORTED.to_string())
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod tests;
