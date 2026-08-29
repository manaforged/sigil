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

#[cfg(any(feature = "desktop", feature = "tui"))]
#[test]
fn copy_paste_roundtrip_when_platform_clipboard_available() {
    install_os();
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
    install(Memory(RefCell::new(None)));
    copy("x").expect("memory stores");
    assert_eq!(paste().expect("memory reads"), "x");
    install(sigil_widgets::NoopClipboard);
}

#[test]
fn copy_without_handle_errors() {
    install(sigil_widgets::NoopClipboard);
    let err = copy("x").unwrap_err();
    assert!(
        err.to_string().contains("installed"),
        "expected install error, got {err}"
    );
}
