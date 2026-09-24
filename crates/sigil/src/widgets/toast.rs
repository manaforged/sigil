use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::prelude::{JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{Signal, derived};
use crate::style::{DisplayKind, NodeStyleExt, StyleAtom, Token, space};

use super::popup::SURFACE;

#[must_use]
pub fn toast(message: Signal<Option<String>>) -> NodeBuilder {
    let body = derived(move || message.get().unwrap_or_default());
    container()
        .role(Role::Status)
        .label(body)
        .layout(
            LayoutStyle::flex_row()
                .position(Position::Absolute)
                .inset_bottom(space(4))
                .inset_left(space(4))
                .inset_right(space(4))
                .justify_content(JustifyContent::Center)
                .padding(space(3)),
        )
        .class(SURFACE)
        .style_reactive(move || {
            if message.get().is_some() {
                &[]
            } else {
                &[StyleAtom::Display(DisplayKind::None)]
            }
        })
        .child(text(body).class(Token::OnSurface.text()))
}

#[cfg(test)]
#[path = "toast_tests.rs"]
mod tests;
