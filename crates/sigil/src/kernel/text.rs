use std::fmt;
use std::rc::Rc;

use crate::kernel::Role;
use crate::reactive::{Signal, effect_named, memo};

use crate::kernel::NodeBuilder;
use crate::kernel::component::NodePatch;
use crate::kernel::hooks::NodeHooks;
use crate::kernel::{A11yProps, Cx, DeltaKind, IntoNode, NodeId, NodeType};
use crate::style::{Paint, StateStyleBuilder};

/// Static, signal, or computed text.
#[derive(Clone)]
pub enum TextContent {
    /// Text that does not change.
    Static(String),
    /// Text read from a signal.
    Reactive(Signal<String>),
    /// Text that a closure returns.
    Compute(Rc<dyn Fn() -> String>),
}

impl TextContent {
    /// The text when it is static; `None` for signal and computed text.
    #[must_use]
    pub fn as_static_str(&self) -> Option<&str> {
        match self {
            Self::Static(value) => Some(value),
            Self::Reactive(_) | Self::Compute(_) => None,
        }
    }

    /// The current text, read without subscribing; a disposed signal reads as empty.
    #[must_use]
    pub fn resolve(&self) -> String {
        self.with_str(str::to_owned)
    }

    /// Calls `f` with the current text without subscribing; a disposed signal reads as empty.
    pub(crate) fn with_str<R>(&self, mut f: impl FnMut(&str) -> R) -> R {
        match self {
            Self::Static(value) => f(value),
            Self::Reactive(signal) => signal
                .try_peek_with(|value| f(value))
                .unwrap_or_else(|| f("")),
            Self::Compute(compute) => f(&compute()),
        }
    }

    pub(crate) fn hash_into(&self, hasher: &mut impl std::hash::Hasher) {
        use std::hash::Hash as _;
        match self {
            Self::Reactive(signal) => signal.stamp().hash(hasher),
            Self::Static(_) | Self::Compute(_) => self.with_str(|value| value.hash(hasher)),
        }
    }

    /// Writes the current text to `out` and returns whether it changed.
    pub fn refresh(&self, out: &mut String) -> bool {
        self.with_str(|value| replace(out, value))
    }

    pub(crate) fn live(&self) -> Option<Signal<String>> {
        match self {
            Self::Static(_) => None,
            Self::Reactive(signal) => Some(*signal),
            Self::Compute(compute) => {
                let compute = Rc::clone(compute);
                Some(memo(move || compute()))
            }
        }
    }

    fn into_live(self) -> Self {
        self.live().map_or(self, Self::Reactive)
    }
}

impl fmt::Debug for TextContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static(value) => f.debug_tuple("Static").field(value).finish(),
            Self::Reactive(_) => f.write_str("Reactive"),
            Self::Compute(_) => f.write_str("Compute"),
        }
    }
}

impl From<&str> for TextContent {
    fn from(value: &str) -> Self {
        Self::Static(value.to_owned())
    }
}

impl From<String> for TextContent {
    fn from(value: String) -> Self {
        Self::Static(value)
    }
}

impl From<&String> for TextContent {
    fn from(value: &String) -> Self {
        Self::Static(value.clone())
    }
}

impl From<std::borrow::Cow<'_, str>> for TextContent {
    fn from(value: std::borrow::Cow<'_, str>) -> Self {
        Self::Static(value.into_owned())
    }
}

impl<T: fmt::Display + 'static> From<Signal<T>> for TextContent {
    fn from(value: Signal<T>) -> Self {
        if let Some(text) = (&value as &dyn std::any::Any).downcast_ref::<Signal<String>>() {
            return Self::Reactive(*text);
        }
        Self::Reactive(memo(move || value.with(ToString::to_string)))
    }
}

impl<F, S> From<F> for TextContent
where
    F: Fn() -> S + 'static,
    S: Into<String>,
{
    fn from(value: F) -> Self {
        Self::Compute(Rc::new(move || value().into()))
    }
}

/// Builder of a text node.
#[must_use]
#[derive(Debug)]
pub struct TextBuilder {
    pub(crate) node: NodeBuilder,
    pub(crate) hooks: NodeHooks,
}

/// Creates a text node that wraps and shows no ellipsis.
pub fn text(source: impl Into<TextContent>) -> TextBuilder {
    let kind = NodeType::Text {
        content: source.into(),
        wrap: true,
        ellipsis: false,
    };
    let a11y = A11yProps {
        role: Role::Label,
        ..A11yProps::default()
    };
    TextBuilder {
        node: NodeBuilder::from_parts(kind, a11y, Vec::new()),
        hooks: NodeHooks::default(),
    }
}

impl TextBuilder {
    /// Turns wrapping on and the ellipsis off.
    pub const fn wrap(mut self) -> Self {
        if let NodeType::Text { wrap, ellipsis, .. } = &mut self.node.kind {
            *wrap = true;
            *ellipsis = false;
        }
        self
    }

    /// Turns wrapping off.
    pub const fn nowrap(mut self) -> Self {
        if let NodeType::Text { wrap, .. } = &mut self.node.kind {
            *wrap = false;
        }
        self
    }

    /// Turns the ellipsis on and wrapping off.
    pub const fn ellipsize(mut self) -> Self {
        if let NodeType::Text { wrap, ellipsis, .. } = &mut self.node.kind {
            *ellipsis = true;
            *wrap = false;
        }
        self
    }

    shared_node_methods!(node);

    /// Sets the runs.
    pub(crate) fn runs(self, runs: Vec<crate::style::TextRun>) -> Self {
        if runs.is_empty() {
            return self;
        }
        let runs = crate::style::TextRuns(runs.into());
        self.mutate(move |node| {
            node.attach(runs);
        })
    }
}

impl Paint for TextBuilder {
    fn paint_mut(&mut self) -> &mut StateStyleBuilder {
        self.node.paint_mut()
    }
}

impl IntoNode for TextBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.hooks.finish(self.node).build(cx)
    }

    fn patch(self, cx: &mut Cx, existing: NodeId) -> NodePatch<Self> {
        self.hooks
            .finish(self.node)
            .patch(cx, existing)
            .map(|node| Self {
                node,
                hooks: NodeHooks::default(),
            })
    }
}

impl NodeType {
    pub(crate) fn into_live(self) -> Self {
        match self {
            Self::Text {
                content,
                wrap,
                ellipsis,
            } => Self::Text {
                content: content.into_live(),
                wrap,
                ellipsis,
            },
            other => other,
        }
    }
}

pub(crate) fn track_delta(cx: &Cx, id: NodeId) {
    let Some(node) = cx.get(id) else { return };
    let NodeType::Text {
        content: TextContent::Reactive(signal),
        ..
    } = &node.kind
    else {
        return;
    };
    if let Some(tag) = node.tag() {
        let signal = *signal;
        effect_named("sigil-kernel::text_delta", move || {
            if signal.try_track() {
                tag.record(DeltaKind::Layout);
            }
        });
    }
}

fn replace(out: &mut String, value: &str) -> bool {
    if out == value {
        return false;
    }
    out.clear();
    out.push_str(value);
    true
}

#[cfg(test)]
#[path = "text_patch_tests.rs"]
mod patch_tests;
