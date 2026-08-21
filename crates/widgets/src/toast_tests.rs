use super::*;
use sigil_kernel::Cx;
use sigil_reactive::{Runtime, signal};

#[test]
fn toast_builds_from_a_message_signal() {
    Runtime::scope("toast-builds", || {
        let message = signal(Some(String::from("Saved")));
        let mut cx = Cx::new_root();
        let id = cx.add(toast(message));
        assert!(cx.get(id).is_some());
        message.set(None);
        assert!(message.get().is_none());
    });
}
