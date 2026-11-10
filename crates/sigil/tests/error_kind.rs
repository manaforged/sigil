#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::ErrorKind;

#[test]
fn copy_keeps_the_clipboard_kind() {
    sigil::clipboard::install(sigil::clipboard::NoopClipboard);
    let err = sigil::clipboard::copy("x").expect_err("no clipboard is installed");
    assert_ne!(
        err.kind(),
        ErrorKind::Io,
        "copy mapped every clipboard failure to Io: {err}"
    );
    assert_eq!(err.kind(), ErrorKind::Clipboard);
    assert!(err.to_string().contains("installed"), "{err}");
}
