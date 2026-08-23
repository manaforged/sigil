use super::*;
use std::cell::RefCell;

struct Memory(RefCell<Option<String>>);

impl Clipboard for Memory {
    fn set(&self, text: &str) -> Result<(), sigil_widgets::ClipError> {
        *self.0.borrow_mut() = Some(text.to_owned());
        Ok(())
    }

    fn get(&self) -> Option<String> {
        self.0.borrow().clone()
    }
}

/// On Windows CI / dev rigs the system clipboard is always available;
/// on headless Linux runners arboard returns an init error. Skip
/// the roundtrip there rather than failing the test suite.
#[cfg(any(feature = "desktop", feature = "tui"))]
#[test]
fn copy_paste_roundtrip_when_platform_clipboard_available() {
    install();
    let probe = copy("");
    if probe.is_err() {
        eprintln!(
            "skipping clipboard roundtrip — platform clipboard unavailable: {:?}",
            probe.err()
        );
        return;
    }
    let payload = "sigil clipboard test \u{1f5d2}";
    copy(payload).expect("copy succeeds after probe");
    let read_back = paste().expect("paste succeeds after copy");
    assert_eq!(read_back, payload);
}

#[test]
fn copy_uses_the_installed_handle() {
    set_clipboard(Memory(RefCell::new(None)));
    copy("x").expect("memory stores");
    assert_eq!(paste().expect("memory reads"), "x");
    set_clipboard(sigil_widgets::NoopClipboard);
}

#[test]
fn copy_without_handle_errors() {
    set_clipboard(sigil_widgets::NoopClipboard);
    let err = copy("x").unwrap_err();
    assert!(
        err.to_string().contains("set_clipboard"),
        "expected install error, got {err}"
    );
}
