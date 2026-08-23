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
        .class("rounded-md shadow-md bg-surface-hover")
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
#[path = "toast_tests.rs"]
mod tests;
