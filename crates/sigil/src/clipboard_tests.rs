use super::*;
use std::cell::RefCell;

struct Memory(RefCell<Option<String>>);

#[cfg(feature = "clipboard-image")]
#[test]
#[ignore = "requires a native clipboard containing the 2x2 RGBA fixture"]
fn native_image_reads_pixels() {
    let source = image()
        .expect("native clipboard read")
        .expect("image present");
    assert_eq!((source.width(), source.height()), (2, 2));
    assert_eq!(
        source.rgba(),
        [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255
        ]
    );
}

#[cfg(feature = "clipboard-image")]
#[test]
#[ignore = "requires a native clipboard containing only text"]
fn native_image_without_pixels() {
    assert_eq!(image().expect("native clipboard read"), None);
}

impl Clipboard for Memory {
    fn set(&self, text: &str) -> Result<(), crate::widgets::ClipError> {
        *self.0.borrow_mut() = Some(text.to_owned());
        Ok(())
    }

    fn get(&self) -> Option<String> {
        self.0.borrow().clone()
    }
}

#[cfg(feature = "desktop")]
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
    install(crate::widgets::NoopClipboard);
}

#[test]
fn copy_without_handle_errors() {
    install(crate::widgets::NoopClipboard);
    let err = copy("x").unwrap_err();
    assert!(
        err.to_string().contains("installed"),
        "expected install error, got {err}"
    );
}
