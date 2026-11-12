use core::convert::Infallible;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::ErrorKind;
use crate::app::Wake;
use crate::app::bridge::Native;
use crate::error::{AppResult, Error};
use crate::hal::events::{KeyPhase, LogicalKey, Modifiers, Shortcut};
use crate::reactive::EventChannel;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

struct Registered {
    hotkey: HotKey,
    channel: EventChannel<KeyPhase>,
}

thread_local! {
    static NATIVE: Native<Infallible, GlobalHotKeyManager, GlobalHotKeyEvent> = Native::new();
    static REGISTERED: RefCell<HashMap<u32, Registered>> = RefCell::new(HashMap::new());
}

const MODIFIER_TOKENS: [(Modifiers, &str); 4] = [
    (Modifiers::SHIFT, "shift"),
    (Modifiers::CTRL, "control"),
    (Modifiers::ALT, "alt"),
    (Modifiers::META, "super"),
];

/// Registers a global shortcut and returns the channel of its key phases.
pub fn register(shortcut: &Shortcut) -> AppResult<EventChannel<KeyPhase>> {
    let hotkey = to_hotkey(shortcut)?;
    if let Some(channel) = channel_of(hotkey.id()) {
        return Ok(channel);
    }
    with_manager(|manager| manager.register(hotkey))?;
    let channel = EventChannel::new();
    REGISTERED.with(|map| {
        map.borrow_mut().insert(
            hotkey.id(),
            Registered {
                hotkey,
                channel: channel.clone(),
            },
        );
    });
    Ok(channel)
}

/// Removes a global shortcut and returns whether it was registered.
pub fn unregister(shortcut: &Shortcut) -> AppResult<bool> {
    let id = to_hotkey(shortcut)?.id();
    let Some(entry) = REGISTERED.with(|map| map.borrow_mut().remove(&id)) else {
        return Ok(false);
    };
    with_manager(|manager| manager.unregister(entry.hotkey))?;
    Ok(true)
}

pub(crate) fn install(wake: impl Fn() -> Wake) {
    if let Some(sink) = NATIVE.with(|native| native.sink_once(&wake)) {
        GlobalHotKeyEvent::set_event_handler(Some(sink));
    }
}

pub(crate) fn pump() {
    let pending: Vec<GlobalHotKeyEvent> = NATIVE.with(|native| {
        native
            .drain()
            .chain(GlobalHotKeyEvent::receiver().try_iter())
            .collect()
    });
    for event in pending {
        let phase = match event.state() {
            HotKeyState::Pressed => KeyPhase::Down,
            HotKeyState::Released => KeyPhase::Up,
        };
        if let Some(channel) = channel_of(event.id()) {
            channel.emit(phase);
        }
    }
}

fn channel_of(id: u32) -> Option<EventChannel<KeyPhase>> {
    REGISTERED.with(|map| map.borrow().get(&id).map(|entry| entry.channel.clone()))
}

fn with_manager(
    op: impl FnOnce(&GlobalHotKeyManager) -> global_hotkey::Result<()>,
) -> AppResult<()> {
    NATIVE.with(|native| {
        native.installed(|slot| -> AppResult<()> {
            let manager = match &mut *slot {
                Some(manager) => manager,
                empty => empty.insert(
                    GlobalHotKeyManager::new()
                        .map_err(|err| Error::new(crate::ErrorKind::Other, err))?,
                ),
            };
            op(manager).map_err(|err| Error::new(crate::ErrorKind::Other, err))
        })
    })
}

fn to_hotkey(shortcut: &Shortcut) -> AppResult<HotKey> {
    let mut text: String = MODIFIER_TOKENS
        .iter()
        .filter(|(flag, _)| shortcut.modifiers.contains(*flag))
        .fold(String::new(), |mut text, (_, token)| {
            text.push_str(token);
            text.push('+');
            text
        });
    match &shortcut.key {
        LogicalKey::Character(value) if value.as_str() == " " => text.push_str("Space"),
        LogicalKey::Character(value) => text.push_str(value.as_str()),
        LogicalKey::Named(named) => text.push_str(&named.to_string()),
    }
    text.parse::<HotKey>()
        .map_err(|err| Error::new(ErrorKind::Config, err))
}

#[cfg(test)]
#[path = "hotkey_tests.rs"]
mod tests;
