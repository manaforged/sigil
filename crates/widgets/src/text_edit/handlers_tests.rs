use super::{TextEditRuntime, noop_change};
use crate::clipboard::clipboard;
use sigil_reactive::{Runtime, signal};

#[test]
fn external_caret_controls_the_next_edit() {
    Runtime::scope("external-caret", || {
        let value = signal(String::new());
        let caret = signal(0);
        let runtime = TextEditRuntime::new(
            value,
            None,
            noop_change(),
            noop_change(),
            clipboard(),
            signal(true),
            Some(caret),
        );
        value.set("café".to_string());
        caret.set("café".len());
        let mut edit = runtime.edit_mut();
        edit.insert_str("\n");
        assert_eq!(edit.as_str(), "café\n");
        assert!(edit.undo());
        assert_eq!(edit.as_str(), "café");
    });
}
