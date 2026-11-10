use std::fmt;
use std::rc::Rc;

use crate::events::Shortcut;
use crate::kernel::Role;
use crate::reactive::{Signal, derived, effect_named};

use crate::kernel::NodeBuilder;
use crate::kernel::component::Patch;
use crate::kernel::hooks::NodeHooks;
use crate::kernel::{A11yProps, AccessibleName, Cx, DeltaKind, IntoNode, NodeId, NodeType};
use crate::style::{PaintTarget, StyleSlot};

/// Static, signal, or computed text.
#[derive(Clone)]
pub enum TextContent {
    /// The static text content.
    Static(String),
    /// The reactive text content.
    Reactive(Signal<String>),
    /// The compute text content.
    Compute(Rc<dyn Fn() -> String>),
}

impl TextContent {
    /// Returns the as static str.
    #[must_use]
    pub fn as_static_str(&self) -> Option<&str> {
        match self {
            Self::Static(value) => Some(value),
            Self::Reactive(_) | Self::Compute(_) => None,
        }
    }

    /// Returns the current text without subscribing.
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
        self.with_str(|value| value.hash(hasher));
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
                Some(derived(move || compute()))
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

impl<T: fmt::Display + 'static> From<Signal<T>> for TextContent {
    fn from(value: Signal<T>) -> Self {
        if let Some(text) = (&value as &dyn std::any::Any).downcast_ref::<Signal<String>>() {
            return Self::Reactive(*text);
        }
        Self::Reactive(derived(move || value.with(ToString::to_string)))
    }
}

impl<F: Fn() -> String + 'static> From<F> for TextContent {
    fn from(value: F) -> Self {
        Self::Compute(Rc::new(value))
    }
}

/// Builder of a text node.
#[derive(Debug)]
pub struct TextBuilder {
    pub(crate) node: NodeBuilder,
    pub(crate) hooks: NodeHooks,
}

/// The one text node.
#[must_use]
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
    /// Applies the wrap option.
    #[must_use]
    pub const fn wrap(mut self) -> Self {
        if let NodeType::Text { wrap, ellipsis, .. } = &mut self.node.kind {
            *wrap = true;
            *ellipsis = false;
        }
        self
    }

    /// Applies the nowrap option.
    #[must_use]
    pub const fn nowrap(mut self) -> Self {
        if let NodeType::Text { wrap, .. } = &mut self.node.kind {
            *wrap = false;
        }
        self
    }

    /// Applies the ellipsize option.
    #[must_use]
    pub const fn ellipsize(mut self) -> Self {
        if let NodeType::Text { wrap, ellipsis, .. } = &mut self.node.kind {
            *ellipsis = true;
            *wrap = false;
        }
        self
    }

    /// Sets the role.
    #[must_use]
    pub const fn role(mut self, role: Role) -> Self {
        self.node.a11y.role = role;
        self
    }

    /// Sets the label.
    #[must_use]
    pub fn label(mut self, name: impl Into<AccessibleName>) -> Self {
        self.node = self.node.label(name);
        self
    }

    /// Sets the description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.node = self.node.description(description);
        self
    }

    /// Sets the focusable.
    #[must_use]
    pub const fn focusable(mut self, yes: bool) -> Self {
        self.node.a11y.focusable = yes;
        self
    }

    /// Sets the tab index.
    #[must_use]
    pub const fn tab_index(mut self, index: i32) -> Self {
        self.node.a11y.tab_index = Some(index);
        self
    }

    /// Sets the keyboard shortcut.
    #[must_use]
    pub fn keyboard_shortcut(mut self, shortcut: impl Into<Shortcut>) -> Self {
        self.node = self.node.keyboard_shortcut(shortcut);
        self
    }

    /// Sets the hittable.
    #[must_use]
    pub const fn hittable(mut self, yes: bool) -> Self {
        self.node.a11y.hittable = yes;
        self
    }

    /// Adds this text node to the tree and returns its id.
    #[allow(
        clippy::should_implement_trait,
        reason = "intentional, reviewed trade-off"
    )]
    pub fn add(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }

    /// Sets the runs.
    #[must_use]
    pub(crate) fn runs(self, runs: Vec<crate::style::TextRun>) -> Self {
        if runs.is_empty() {
            return self;
        }
        let runs = crate::style::TextRuns(runs.into());
        self.mutate(move |node| {
            node.attach(runs);
        })
    }

    /// Sets the mutate.
    #[must_use]
    pub fn mutate(mut self, apply: impl FnOnce(&mut crate::kernel::Node) + 'static) -> Self {
        self.node = self.node.mutate(apply);
        self
    }
}

impl PaintTarget for TextBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        self.node.style_slot()
    }
}

impl IntoNode for TextBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.hooks.finish(self.node).build(cx)
    }

    fn patch(self, cx: &mut Cx, existing: NodeId) -> Patch<Self> {
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
