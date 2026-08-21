use super::*;

/// On Windows CI / dev rigs the system clipboard is always available;
/// on headless Linux runners arboard returns an init error. Skip
/// the roundtrip there rather than failing the test suite.
#[test]
fn copy_paste_roundtrip_when_platform_clipboard_available() {
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
