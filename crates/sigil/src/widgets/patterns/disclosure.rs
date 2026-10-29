use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, Role, container, show};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;

use super::activatable::activatable;
use super::list_row::{Disclosure, disclosure_slot};

#[must_use]
pub fn disclosure<H, B>(
    open: Signal<bool>,
    header: impl Fn(bool) -> H + 'static,
    body: impl Fn() -> B + 'static,
) -> NodeBuilder
where
    H: IntoNode + 'static,
    B: IntoNode + 'static,
{
    let header = Rc::new(header);
    let shown = Rc::clone(&header);
    let head = activatable(
        container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .min_size(0.0, 0.0)
                    .align_items(AlignItems::Center),
            )
            .expanded(open)
            .child(show(
                open,
                || disclosure_slot(Disclosure::Expanded),
                || disclosure_slot(Disclosure::Collapsed),
            ))
            .child(show(open, move || shown(true), move || header(false))),
        Role::Button,
        &(Rc::new(move || open.set(!open.peek())) as Rc<dyn Fn()>),
    );
    container()
        .layout(LayoutStyle::flex_column().fill_width().min_size(0.0, 0.0))
        .child(head)
        .child(show(open, body, container))
}
