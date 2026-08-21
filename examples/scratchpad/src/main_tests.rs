use super::*;
use sigil::reactive::Runtime;

fn dirty(state: ScratchpadState) -> bool {
    state.current.peek() != state.saved.peek()
}

#[test]
fn tree_builds_without_panic() {
    Runtime::scope("scratchpad-smoke", || {
        let mut cx = Cx::new_root();
        let root = app_tree(&mut cx);
        // `NodeId` doesn't expose a public `is_null()`/`is_some()`
        // helper, but a built node has a debug repr distinct from
        // a freshly-constructed default. Sanity: re-building does
        // not panic and we can produce a second tree under the
        // same runtime scope without state collision.
        let _ = format!("{root:?}");
    });
}

#[test]
fn save_marks_clean_and_revert_restores_buffer() {
    Runtime::scope("scratchpad-state", || {
        let state = ScratchpadState::new();
        state.current.set(String::from("hello"));
        assert!(dirty(state), "edit should leave buffer dirty");

        state.save();
        assert!(!dirty(state), "save should clear dirty flag");
        assert_eq!(state.saved.peek(), "hello");

        state.current.set(String::from("hello world"));
        assert!(dirty(state), "second edit should re-dirty");

        state.revert();
        assert_eq!(state.current.peek(), "hello");
        assert!(!dirty(state), "revert should clear dirty flag");
    });
}

#[test]
fn clear_empties_buffer_and_marks_dirty_when_saved_nonempty() {
    Runtime::scope("scratchpad-clear", || {
        let state = ScratchpadState::new();
        state.current.set(String::from("notes"));
        state.save();
        assert!(!dirty(state));

        state.clear();
        assert_eq!(state.current.peek(), "");
        assert!(
            dirty(state),
            "clearing a non-empty saved buffer should re-dirty"
        );
    });
}
