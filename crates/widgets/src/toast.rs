//! Toast — a short status line. Pass the message signal.

use sigil_kernel::{NodeBuilder, Role, container, text};
use sigil_layout::prelude::{JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use sigil_reactive::{Signal, derived};
use sigil_style::{DisplayKind, NodeStyleExt, StyleAtom};

/// Show `message` at the bottom of the host when it is `Some`.
///
/// Set the signal to `None` to hide. Same signal-first shape as
/// [`modal`](super::modal).
#[must_use]
pub fn toast(message: Signal<Option<String>>) -> NodeBuilder {
    let body = derived(move || message.get().unwrap_or_default());
    container()
        .role(Role::Status)
        .layout(
            LayoutStyle::flex_row()
                .position(Position::Absolute)
                .inset_bottom(16.0)
                .inset_left(16.0)
                .inset_right(16.0)
                .justify_content(JustifyContent::Center)
                .padding(12.0),
        )
        .class("rounded-md shadow-md bg-surface")
        .style_reactive(move || {
            if message.get().is_some() {
                &[]
            } else {
                &[StyleAtom::Display(DisplayKind::None)]
            }
        })
        .child(text(body))
}

#[cfg(test)]
mod tests {
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
}
