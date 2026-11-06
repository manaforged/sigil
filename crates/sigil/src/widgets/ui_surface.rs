use std::rc::Rc;

use serde_json::Value;

use crate::kernel::{NodeBuilder, container};
use crate::layout::LayoutStyle;
use crate::reactive::{Reactive, Signal, signal, untrack};
use crate::style::{Tone, space};
use crate::widgets::layout_ext::LayoutOverrides;
use crate::widgets::{Widget, markdown, notice_band};

mod document;
mod glyph;
mod inputs;
mod media;
mod model;
mod render;
mod spec;

use render::{Handler, Render};

const MAX_SHOWN_SOURCE: usize = 16 << 10;

/// A button event from a UI surface: the action name and its context resolved against the data model.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceAction {
    /// The `action.event.name` of the button.
    pub name: String,
    /// The `action.event.context` object with each bound path replaced by its current value.
    pub context: Value,
}

/// Builder of a UI surface that renders A2UI v0.9 messages with native widgets.
pub struct SurfaceBuilder {
    source: Signal<String>,
    handler: Option<Handler>,
    layout_overrides: LayoutOverrides,
}

/// A UI surface from A2UI v0.9 JSON: one message, an array of messages, or `{"messages": [..]}`.
#[must_use]
pub fn ui_surface(json: impl Reactive<String>) -> SurfaceBuilder {
    SurfaceBuilder {
        source: json.into_signal(),
        handler: None,
        layout_overrides: LayoutOverrides::default(),
    }
}

fn fence(source: &str) -> String {
    let shown = if source.len() > MAX_SHOWN_SOURCE {
        let end = (0..=MAX_SHOWN_SOURCE)
            .rev()
            .find(|index| source.is_char_boundary(*index))
            .unwrap_or(0);
        format!("{}\n…", &source[..end])
    } else {
        source.to_string()
    };
    let mut longest = 0;
    let mut run = 0;
    for byte in shown.bytes() {
        run = if byte == b'`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let ticks = "`".repeat(longest.max(2) + 1);
    format!("{ticks}json\n{}\n{ticks}", shown.trim_end())
}

fn invalid(message: &str, source: &str) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(space(1)))
        .child(notice_band(message.to_string(), Tone::Warn))
        .child(markdown(signal(fence(source))))
}

fn surface(source: &str, handler: Option<Handler>) -> NodeBuilder {
    match document::parse(source) {
        Ok(parsed) => untrack(|| Render::new(parsed, handler).node("root")),
        Err(problem) => invalid(&problem.message(), source),
    }
}

impl SurfaceBuilder {
    /// Calls `f` with the action each time a button with an `action.event` is pressed.
    pub fn on_action(mut self, f: impl Fn(SurfaceAction) + 'static) -> Self {
        self.handler = Some(Rc::new(f));
        self
    }

    fn root(self) -> NodeBuilder {
        let (source, handler) = (self.source, self.handler);
        container()
            .layout(LayoutStyle::flex_column().fill_width())
            .children_reactive(move || [source.with(|json| surface(json, handler.clone()))])
    }
}

impl From<SurfaceBuilder> for NodeBuilder {
    fn from(surface: SurfaceBuilder) -> Self {
        surface.into_node_builder()
    }
}

crate::widgets::builder_impls!([] SurfaceBuilder: widget, node(pub));
