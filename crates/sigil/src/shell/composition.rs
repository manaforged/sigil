use smol_str::SmolStr;

use crate::events::ImeEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompositionInput<'a> {
    Start,
    Update(&'a str),
    End(&'a str),
    Text(&'a str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyRoute {
    Host,
    Browser,
}

const IME_PROCESS_KEY_CODE: u32 = 229;

pub(crate) fn composition_events(input: CompositionInput<'_>) -> Vec<ImeEvent> {
    match input {
        CompositionInput::Start => vec![ImeEvent::Enabled],
        CompositionInput::Update("") => vec![ImeEvent::Preedit(SmolStr::default(), None)],
        CompositionInput::Update(text) => vec![ImeEvent::Preedit(
            SmolStr::new(text),
            Some((text.len(), text.len())),
        )],
        CompositionInput::End(text) => {
            let mut out = vec![ImeEvent::Preedit(SmolStr::default(), None)];
            if !text.is_empty() {
                out.push(ImeEvent::Commit(SmolStr::new(text)));
            }
            out
        }
        CompositionInput::Text("") => Vec::new(),
        CompositionInput::Text(text) => {
            vec![ImeEvent::Enabled, ImeEvent::Commit(SmolStr::new(text))]
        }
    }
}

pub(crate) fn key_route(key: &str, key_code: u32, composing: bool) -> KeyRoute {
    if composing || key_code == IME_PROCESS_KEY_CODE || matches!(key, "Process" | "Dead") {
        KeyRoute::Browser
    } else {
        KeyRoute::Host
    }
}

#[cfg(test)]
#[path = "composition_tests.rs"]
mod tests;
