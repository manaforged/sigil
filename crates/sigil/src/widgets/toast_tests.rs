use super::*;
use crate::kernel::Cx;
use crate::reactive::{Runtime, signal};

#[test]
fn toast_builds_from_a_message_signal() {
    Runtime::scope("toast-builds", || {
        let message = signal(Some(String::from("Saved")));
        let mut cx = Cx::new_root();
        let id = cx.add(toast(message));
        let node = cx.get(id).expect("toast root");
        assert!(!node.children().is_empty(), "the toast shows its message");
    });
}
