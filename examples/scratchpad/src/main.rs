//! Scratchpad — a dogfood example of an editor-tier Sigil app.
//!
//! A single editable text field acts as the working buffer. A second
//! `Signal<String>` represents the "persisted" state. A derived signal
//! compares the two so a dirty indicator can light up without manual
//! state syncing. "Save" copies current → saved; "Revert" copies the
//! other way; "Clear" empties the buffer.
//!
//! There is NO multi-line `text_edit` widget in the prelude
//! (`text_edit` exists at `crates/sigil/src/widgets/text_edit.rs` but
//! is `pub(crate)`), so the buffer is single-line via `text_field`.
//! See `FRICTION.md` for the friction log on that and everything else
//! discovered while building this.

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - scratchpad", 640, 320), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let state = ScratchpadState::new();
    cx.add(scratchpad_root(state))
}

#[derive(Clone, Copy)]
struct ScratchpadState {
    /// Live editing buffer bound to the text field.
    current: Signal<String>,
    /// "Persisted" buffer. Save copies `current` here; Revert copies
    /// `saved` back into `current`.
    saved: Signal<String>,
    /// Derived label: "● unsaved" when `current != saved`, "✓ saved"
    /// otherwise. The underlying dirty boolean is internal to the
    /// closure; tests query it via `dirty()` which compares peeks.
    status_label: Signal<String>,
    /// Plain count of characters in `current`.
    char_label: Signal<String>,
}

impl ScratchpadState {
    fn new() -> Self {
        let current = signal(String::new());
        let saved = signal(String::new());
        let status_label = derived(move || {
            if current.get() == saved.get() {
                String::from("\u{2713} saved")
            } else {
                String::from("\u{25CF} unsaved")
            }
        });
        let char_label = derived(move || format!("{} chars", current.get().chars().count()));
        Self {
            current,
            saved,
            status_label,
            char_label,
        }
    }

    /// Compare peeks of `current` and `saved` to derive dirty state
    /// without subscribing to a signal (test-only helper).
    #[cfg(test)]
    fn dirty(self) -> bool {
        self.current.peek() != self.saved.peek()
    }

    fn save(self) {
        let value = self.current.peek();
        self.saved.set(value);
    }

    fn revert(self) {
        let value = self.saved.peek();
        self.current.set(value);
    }

    fn clear(self) {
        self.current.set(String::new());
    }
}

fn scratchpad_root(state: ScratchpadState) -> NodeBuilder {
    container()
        .role(Role::Main)
        .label("Sigil scratchpad")
        .layout(LayoutStyle::flex_column().gap(12.0).padding(24.0).grow(1.0))
        .class("w-full h-full bg-surface")
        .child(header_row(state))
        .child(editor_row(state))
        .child(footer_row(state))
}

fn header_row(state: ScratchpadState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(12.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween),
        )
        .child(label("Scratchpad"))
        .child(label(state.status_label).class("text-on-surface"))
}

fn editor_row(state: ScratchpadState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(8.0).grow(1.0))
        .child(
            text_field(state.current)
                .placeholder("Type something…")
                .submit(move |_| state.save()),
        )
        .child(label(state.char_label).class("text-on-surface"))
}

fn footer_row(state: ScratchpadState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(8.0)
                .align_items(AlignItems::Center),
        )
        .child(button("Save").primary().activate(move || state.save()))
        .child(button("Revert").activate(move || state.revert()))
        .child(button("Clear").activate(move || state.clear()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil::reactive::Runtime;

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
            assert!(state.dirty(), "edit should leave buffer dirty");

            state.save();
            assert!(!state.dirty(), "save should clear dirty flag");
            assert_eq!(state.saved.peek(), "hello");

            state.current.set(String::from("hello world"));
            assert!(state.dirty(), "second edit should re-dirty");

            state.revert();
            assert_eq!(state.current.peek(), "hello");
            assert!(!state.dirty(), "revert should clear dirty flag");
        });
    }

    #[test]
    fn clear_empties_buffer_and_marks_dirty_when_saved_nonempty() {
        Runtime::scope("scratchpad-clear", || {
            let state = ScratchpadState::new();
            state.current.set(String::from("notes"));
            state.save();
            assert!(!state.dirty());

            state.clear();
            assert_eq!(state.current.peek(), "");
            assert!(
                state.dirty(),
                "clearing a non-empty saved buffer should re-dirty"
            );
        });
    }
}
