//! Widget playground — a runnable catalog of every sigil widget.
//!
//! `cargo run -p widget-playground` opens a single window with five
//! categorized sections rendering one instance of every widget the
//! library exposes (plus the interesting disabled variants). Intended
//! for fast manual smoke-testing of hover / focus / disabled state.

#![allow(missing_docs)]

mod section_container;
mod section_display;
mod section_form;
mod section_navigation;
mod section_reactive;

use sigil::prelude::*;
use sigil::widgets::scroll;

use crate::section_container::container_section;
use crate::section_display::display_section;
use crate::section_form::form_section;
use crate::section_navigation::navigation_section;
use crate::section_reactive::reactive_section;

const SECTION_GAP: f32 = 24.0;
const PAGE_PADDING: f32 = 16.0;
const CONTENT_HEIGHT: f32 = 2400.0;
const VIEWPORT_WIDTH: f32 = 1080.0;
const VIEWPORT_HEIGHT: f32 = 760.0;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Widget Playground", 1100, 800), build)
}

fn build(cx: &mut Cx) -> NodeId {
    cx.add(root())
}

fn root() -> NodeBuilder {
    let content = container()
        .role(Role::Main)
        .label("Widget playground")
        .layout(
            LayoutStyle::flex_column()
                .padding(PAGE_PADDING)
                .gap(SECTION_GAP),
        )
        .class(classes!("w-full bg-surface"))
        .child(page_header())
        .child(form_section())
        .child(display_section())
        .child(navigation_section())
        .child(container_section())
        .child(reactive_section());

    let viewport = scroll(content, CONTENT_HEIGHT)
        .width(VIEWPORT_WIDTH)
        .height(VIEWPORT_HEIGHT);
    container()
        .layout(LayoutStyle::flex_column().fill())
        .class(classes!("bg-surface"))
        .child(viewport)
}

fn page_header() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(4.0))
        .child(text("Sigil widget playground").class(classes!("text-xl font-bold")))
        .child(
            text("Every widget the library exposes, one instance each.").class(classes!("text-sm")),
        )
}

/// Wrap a category in a labeled card.
///
/// The header is a plain `text(...)` node (not `patterns::field`, whose
/// label slot is `Into<TextContent>` — that rejects a styled
/// `TextBuilder`), so the title can carry its own `text-lg` class.
pub(crate) fn category(title: &'static str, body: NodeBuilder) -> NodeBuilder {
    let card = container()
        .layout(LayoutStyle::flex_column().gap(12.0).padding(16.0))
        .class(classes!("w-full rounded-md bg-surface-hover"))
        .child(body);
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(8.0))
        .child(text(title).class(classes!("text-lg font-bold")))
        .child(card)
}

/// Horizontal row of widget cells; one cell per child. Each cell is
/// wrapped in a `flex_grow(1.0) + width(0)` sizing container so the row
/// distributes its inline space equally — without this, a single wide
/// cell (e.g. a multi-line `text_edit`) pushes its siblings off the right
/// edge of the section.
pub(crate) fn row(children: Vec<NodeBuilder>) -> NodeBuilder {
    let mut c = container().layout(
        LayoutStyle::flex_row()
            .fill_width()
            .gap(12.0)
            .align_items(AlignItems::Start),
    );
    for child in children {
        c = c.child(
            container()
                .layout(LayoutStyle::flex_column().grow(1.0).width(0.0))
                .child(child),
        );
    }
    c
}

/// Vertical column of widget cells; one cell per child.
pub(crate) fn column(children: Vec<NodeBuilder>) -> NodeBuilder {
    let mut c = container().layout(LayoutStyle::flex_column().gap(8.0).fill_width());
    for child in children {
        c = c.child(child);
    }
    c
}

/// Wrap any `IntoNode` in a `container()` so the result is a
/// `NodeBuilder` (chainable for `.class()` / `.layout()`). Use for
/// `TextBuilder` / `ScrollBuilder` / `TabsBuilder` etc. that return
/// their own builder type but need to slot into a `Vec<NodeBuilder>`.
pub(crate) fn wrap(node: impl IntoNode + 'static) -> NodeBuilder {
    container().child(node)
}
