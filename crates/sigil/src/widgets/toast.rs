use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::Space;
use crate::layout::{JustifyContent, LayoutStyle, Position};
use crate::reactive::{Reactive, memo};
use crate::style::{Token, space};
use crate::widgets::Widget;

use super::popup::{SURFACE, shown_when};

/// One transient message.
pub fn toast(message: impl Reactive<Option<String>>) -> NodeBuilder {
    let message = message.into_signal();
    let body = memo(move || message.get().unwrap_or_default());
    container()
        .role(Role::Status)
        .accessible(body)
        .layout(
            LayoutStyle::flex_row()
                .position(Position::Absolute)
                .inset_bottom(space(4))
                .inset_left(space(4))
                .inset_right(space(4))
                .justify_content(JustifyContent::Center)
                .padding(Space::Md),
        )
        .class(SURFACE)
        .style_reactive(move || shown_when(message.get().is_some()))
        .child(text(body).class(Token::OnSurface.text()))
}

#[cfg(test)]
#[path = "toast_tests.rs"]
mod tests;
