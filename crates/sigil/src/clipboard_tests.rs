#![allow(clippy::unwrap_used, clippy::shadow_unrelated)]

use super::{Clipboard, NoopClipboard, clipboard, copy, install, paste};

struct Memory(std::cell::RefCell<Option<String>>);

impl Clipboard for Memory {
    fn set(&self, text: &str) -> Result<super::Delivery, super::ClipError> {
        *self.0.borrow_mut() = Some(text.to_owned());
        Ok(super::Delivery::Confirmed)
    }

    fn get(&self) -> Option<String> {
        self.0.borrow().clone()
    }
}

#[test]
fn default_is_noop() {
    let clip = clipboard();
    assert!(clip.set("x").is_err());
    assert!(clip.get().is_none());
    let err = super::write(&NoopClipboard, "x").expect_err("noop cannot store");
    assert!(err.to_string().contains("installed"));
    let err = copy("x").unwrap_err();
    assert!(
        err.to_string().contains("installed"),
        "expected install error, got {err}"
    );
}

#[test]
fn install_is_what_fields_read() {
    install(Memory(std::cell::RefCell::new(None)));
    clipboard().set("hi").expect("memory stores");
    assert_eq!(clipboard().get().as_deref(), Some("hi"));
    copy("x").expect("memory stores");
    assert_eq!(paste().expect("memory reads"), "x");
    install(NoopClipboard);
}
