//! Built-in text node primitive.

use accesskit::Role;
use sigil_reactive::Signal;

use crate::{Component, Cx, NameSource, NodeId, NodeKind, Semantics};

/// Static or reactive text content.
#[derive(Clone, Debug)]
pub enum TextSource {
    /// Fixed text known at build time.
    Static(String),
    /// Text driven by a reactive signal.
    Reactive(Signal<String>),
}

impl TextSource {
    /// Borrow the static text when this source is fixed.
    #[must_use]
    pub fn as_static_str(&self) -> Option<&str> {
        match self {
            Self::Static(value) => Some(value),
            Self::Reactive(_) => None,
        }
    }
}

impl From<&str> for TextSource {
    fn from(value: &str) -> Self {
        Self::Static(value.to_owned())
    }
}

impl From<String> for TextSource {
    fn from(value: String) -> Self {
        Self::Static(value)
    }
}

impl From<Signal<String>> for TextSource {
    fn from(value: Signal<String>) -> Self {
        Self::Reactive(value)
    }
}

/// Builder for a text node.
pub struct TextBuilder {
    source: TextSource,
    semantics: Semantics,
}

/// Create a text node builder.
#[must_use]
pub fn text(source: impl Into<TextSource>) -> TextBuilder {
    TextBuilder {
        source: source.into(),
        semantics: Semantics {
            role: Role::Label,
            ..Semantics::default()
        },
    }
}

impl TextBuilder {
    /// Set the accessibility role.
    #[must_use]
    pub const fn role(mut self, role: Role) -> Self {
        self.semantics.role = role;
        self
    }

    /// Set a static accessible label.
    #[must_use]
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.semantics.name = NameSource::Static(label.into());
        self
    }

    /// Set a reactive accessible label.
    #[must_use]
    pub fn aria_label_signal(mut self, signal: Signal<String>) -> Self {
        self.semantics.name = NameSource::Derived(signal);
        self
    }

    /// Set a static accessible description.
    #[must_use]
    pub fn aria_description(mut self, description: impl Into<String>) -> Self {
        self.semantics.description = Some(NameSource::Static(description.into()));
        self
    }

    /// Mark this text node focusable or not.
    #[must_use]
    pub const fn focusable(mut self, yes: bool) -> Self {
        self.semantics.focusable = yes;
        self
    }

    /// Add this builder to a context.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }
}

impl Component for TextBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        cx.insert_raw(
            NodeKind::Text {
                source: self.source,
            },
            self.semantics,
        )
    }
}
