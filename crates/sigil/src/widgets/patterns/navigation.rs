use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::{AlignItems, LayoutStyle, Space};
use crate::reactive::{Reactive, Signal, derived, effect};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::style::{StyleAtom, Token};
use crate::widgets::{Layout, Widget};

use super::super::button::{ButtonBuilder, button};
use super::super::link::link;

const SLOTS: usize = 7;
const GAP: &str = "…";
const SEPARATOR: &str = "/";

const fn slot(index: usize, page: usize, pages: usize) -> Option<usize> {
    if pages <= SLOTS {
        return Some(index);
    }
    let last = pages - 1;
    let (head, tail) = (page <= 3, page + 4 >= pages);
    match index {
        0 => Some(0),
        6 => Some(last),
        1 if !head => None,
        5 if !tail => None,
        _ if head => Some(index),
        _ if tail => Some(pages - SLOTS + index),
        _ => Some(page + index - 3),
    }
}

static CURRENT: &[StyleAtom] = &[Token::Accent4.background(), Token::Accent12.text()];
static OTHER: &[StyleAtom] = &[];

fn row_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .gap(Space::Xs)
        .align_items(AlignItems::Center)
}

/// Page buttons with previous and next; `page` holds the zero-based current page.
///
/// More than seven pages collapse to the first, the last, and the pages around `page`.
///
/// ```
/// use sigil::prelude::*;
/// use sigil::widgets::pagination;
///
/// fn results() -> impl IntoNode {
///     let page = signal(0_usize);
///     let pages = signal(12_usize);
///     pagination(page, pages)
/// }
/// ```
#[must_use]
pub fn pagination(page: Signal<usize>, pages: impl Reactive<usize>) -> NodeBuilder {
    let pages = pages.into_signal();
    let count = derived(move || pages.get().min(SLOTS));
    let last = derived(move || pages.get().saturating_sub(1));
    effect(move || {
        let last = last.get();
        if page.peek() > last {
            page.set(last);
        }
    });
    container()
        .role(Role::Navigation)
        .label(theme_label(StringKey::Pagination))
        .layout(row_layout())
        .child(
            button("‹")
                .ghost()
                .accessible(theme_label(StringKey::PreviousPage))
                .disabled(derived(move || page.get() == 0))
                .activate(move || page.update(|at| *at = at.saturating_sub(1))),
        )
        .child(container().layout(row_layout()).children_reactive(move || {
            (0..count.get()).map(move |index| page_button(index, page, pages))
        }))
        .child(
            button("›")
                .ghost()
                .accessible(theme_label(StringKey::NextPage))
                .disabled(derived(move || page.get() >= last.get()))
                .activate(move || page.update(|at| *at = (*at + 1).min(last.peek()))),
        )
}

fn page_button(index: usize, page: Signal<usize>, pages: Signal<usize>) -> ButtonBuilder {
    let target = derived(move || slot(index, page.get(), pages.get()));
    let current = derived(move || target.get() == Some(page.get()));
    button(move || {
        target
            .get()
            .map_or_else(|| GAP.to_owned(), |at| (at + 1).to_string())
    })
    .ghost()
    .disabled(derived(move || target.get().is_none()))
    .activate(move || {
        if let Some(at) = target.peek() {
            page.set(at);
        }
    })
    .map_node(move |node| {
        node.selected(current)
            .style_reactive(move || if current.get() { CURRENT } else { OTHER })
    })
}

/// A path of segments; each one but the last is a link that runs `activate` with its index.
///
/// ```
/// use sigil::prelude::*;
/// use sigil::widgets::breadcrumbs;
///
/// fn trail() -> impl IntoNode {
///     let path = signal(vec!["Home".to_owned(), "Docs".to_owned(), "Install".to_owned()]);
///     breadcrumbs(path, move |at| path.update(|segments| segments.truncate(at + 1)))
/// }
/// ```
#[must_use]
pub fn breadcrumbs(
    segments: impl Reactive<Vec<String>>,
    activate: impl Fn(usize) + 'static,
) -> NodeBuilder {
    let segments = segments.into_signal();
    let activate = std::rc::Rc::new(activate);
    container()
        .role(Role::Navigation)
        .label(theme_label(StringKey::Breadcrumbs))
        .layout(row_layout())
        .children_reactive(move || {
            let names = segments.get();
            let last = names.len().saturating_sub(1);
            let mut nodes: Vec<NodeBuilder> = Vec::with_capacity(names.len() * 2);
            for (index, name) in names.into_iter().enumerate() {
                if index > 0 {
                    nodes.push(separator());
                }
                nodes.push(if index == last {
                    container().child(
                        text(name)
                            .nowrap()
                            .class(Token::OnSurface.text())
                            .class("font-medium"),
                    )
                } else {
                    let activate = std::rc::Rc::clone(&activate);
                    link(name)
                        .activate(move || activate(index))
                        .into_node_builder()
                });
            }
            nodes
        })
}

fn separator() -> NodeBuilder {
    container().child(text(SEPARATOR).class(Token::Gray11.text()).hittable(false))
}
