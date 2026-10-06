use std::cell::RefCell;
use std::collections::HashMap;

use crate::app::ErrorKind;
use crate::app::error::{AppResult, Error};
use crate::hal::events::{KeyPhase, LogicalKey, ModifiersState, Shortcut};
use crate::reactive::EventChannel;
use crossbeam_channel::{Receiver, Sender, unbounded};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

struct Registered {
    hotkey: HotKey,
    channel: EventChannel<KeyPhase>,
}

thread_local! {
    static MANAGER: RefCell<Option<GlobalHotKeyManager>> = const { RefCell::new(None) };
    static REGISTERED: RefCell<HashMap<u32, Registered>> = RefCell::new(HashMap::new());
    static BRIDGE: (Sender<GlobalHotKeyEvent>, Receiver<GlobalHotKeyEvent>) = unbounded();
}

const MODIFIER_TOKENS: [(ModifiersState, &str); 4] = [
    (ModifiersState::SHIFT, "shift"),
    (ModifiersState::CONTROL, "control"),
    (ModifiersState::ALT, "alt"),
    (ModifiersState::META, "super"),
];

pub fn register(shortcut: &Shortcut) -> AppResult<EventChannel<KeyPhase>> {
    let hotkey = to_hotkey(shortcut)?;
    if let Some(channel) = REGISTERED.with(|map| {
        map.borrow()
            .get(&hotkey.id())
            .map(|entry| entry.channel.clone())
    }) {
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

pub fn unregister(shortcut: &Shortcut) -> AppResult<bool> {
    let id = to_hotkey(shortcut)?.id();
    let Some(entry) = REGISTERED.with(|map| map.borrow_mut().remove(&id)) else {
        return Ok(false);
    };
    with_manager(|manager| manager.unregister(entry.hotkey))?;
    Ok(true)
}

pub(crate) fn install(wake: impl Fn() + Send + Sync + 'static) {
    let tx = BRIDGE.with(|(tx, _)| tx.clone());
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        let _ = tx.send(event);
        wake();
    }));
}

pub(crate) fn pump() {
    let pending: Vec<GlobalHotKeyEvent> = BRIDGE.with(|(_, rx)| {
        rx.try_iter()
            .chain(GlobalHotKeyEvent::receiver().try_iter())
            .collect()
    });
    for event in pending {
        let phase = match event.state() {
            HotKeyState::Pressed => KeyPhase::Down,
            HotKeyState::Released => KeyPhase::Up,
        };
        let channel = REGISTERED.with(|map| {
            map.borrow()
                .get(&event.id())
                .map(|entry| entry.channel.clone())
        });
        if let Some(channel) = channel {
            channel.emit(phase);
        }
    }
}

fn with_manager(
    op: impl FnOnce(&GlobalHotKeyManager) -> global_hotkey::Result<()>,
) -> AppResult<()> {
    MANAGER.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = Some(GlobalHotKeyManager::new().map_err(Error::new)?);
        }
        slot.as_ref().map_or_else(
            || Err(unsupported()),
            |manager| op(manager).map_err(Error::new),
        )
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
        .map_err(|err| Error::with_kind(ErrorKind::Config, err))
}

fn unsupported() -> Error {
    Error::new(std::io::Error::from(std::io::ErrorKind::Unsupported))
}

#[cfg(test)]
#[path = "hotkey_tests.rs"]
mod tests;
