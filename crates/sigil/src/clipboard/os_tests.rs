#[cfg(feature = "clipboard-image")]
use crate::clipboard::image;
#[cfg(feature = "desktop")]
use crate::clipboard::{copy, install_os, paste};

#[cfg(feature = "clipboard-image")]
#[test]
#[ignore = "manual: needs a native clipboard containing the 2x2 RGBA fixture"]
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
#[ignore = "manual: needs a native clipboard containing only text"]
fn native_image_without_pixels() {
    assert_eq!(image().expect("native clipboard read"), None);
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
