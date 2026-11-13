use crate::layout::Space;
use std::rc::Rc;

use crate::kernel::{NodeBuilder, container};
use crate::layout::LayoutStyle;
use crate::reactive::{Reactive, Signal, signal, untrack};
use crate::style::{Style, Tone};
use crate::widgets::layout_ext::LayoutOverrides;
use crate::widgets::{Layout, markdown, notice_band};

use super::shared::caller_style;

mod document;
mod glyph;
mod inputs;
mod media;
mod model;
mod pieces;
mod prose;
mod render;
mod spec;

use render::{Handler, Render};

const MAX_SHOWN_SOURCE: usize = 16 << 10;

/// The A2UI 1.0 catalog of Sigil components (Chart, Stat, Table, Markdown, Mermaid, Diff, Item), as JSON.
pub const A2UI_CATALOG: &str = include_str!("a2ui/catalog.json");

/// Builder of an A2UI 1.0 or v0.9 surface rendered with native widgets.
#[must_use]
pub struct A2uiBuilder {
    source: Signal<String>,
    handler: Option<Handler>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

impl core::fmt::Debug for A2uiBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("A2uiBuilder")
            .field("source", &self.source)
            .field("style_accumulator", &self.style_accumulator)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// An A2UI surface from JSON: one message, an array of messages, or `{"messages": [..]}`.
pub fn a2ui(json: impl Reactive<String>) -> A2uiBuilder {
    A2uiBuilder {
        source: json.into_signal(),
        handler: None,
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::default(),
    }
}

fn fence(source: &str) -> String {
    let shown = if source.len() > MAX_SHOWN_SOURCE {
        let end = source.floor_char_boundary(MAX_SHOWN_SOURCE);
        format!("{}\n…", &source[..end])
    } else {
        source.to_string()
    };
    let ticks = "`".repeat(prose::longest_tick_run(&shown).max(2) + 1);
    format!("{ticks}json\n{}\n{ticks}", shown.trim_end())
}

fn invalid(message: &str, source: &str) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(Space::Xs))
        .child(notice_band(message.to_string(), Tone::Warning))
        .child(markdown(signal(fence(source))))
}

fn surface(source: &str, handler: Option<Handler>) -> NodeBuilder {
    match document::parse(source) {
        Ok(parsed) => untrack(|| Render::new(parsed, handler).node("root")),
        Err(problem) => invalid(&problem.message(), source),
    }
}

impl A2uiBuilder {
    /// Calls `activate` with a pressed button's action name and its context as JSON.
    pub fn activate(mut self, activate: impl Fn(&str, &str) + 'static) -> Self {
        self.handler = Some(Rc::new(activate));
        self
    }

    fn root(self) -> NodeBuilder {
        let (source, handler) = (self.source, self.handler);
        let node = container()
            .layout(LayoutStyle::flex_column().fill_width())
            .children_reactive(move || [source.with(|json| surface(json, handler.clone()))]);
        caller_style(node, self.style_accumulator)
    }
}

impl From<A2uiBuilder> for NodeBuilder {
    fn from(surface: A2uiBuilder) -> Self {
        surface.into_node_builder()
    }
}

crate::widgets::builder_impls!([] A2uiBuilder: paint(style_accumulator), widget, node(pub));
