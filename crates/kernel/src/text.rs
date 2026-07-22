//! Built-in text node primitive.

use accesskit::Role;
use sigil_hal::Shortcut;
use sigil_reactive::Signal;

use crate::{A11yProps, AccessibleName, Cx, IntoNode, NodeId, NodeType};

type TextMutator = Box<dyn FnOnce(&mut crate::Node)>;

/// Static or reactive text content.
#[derive(Clone, Debug)]
pub enum TextContent {
    /// Fixed text known at build time.
    Static(String),
    /// Text driven by a reactive signal.
    Reactive(Signal<String>),
}

impl TextContent {
    /// Borrow the static text when this source is fixed.
    #[must_use]
    pub fn as_static_str(&self) -> Option<&str> {
        match self {
            Self::Static(value) => Some(value),
            Self::Reactive(_) => None,
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

impl From<Signal<String>> for TextContent {
    fn from(value: Signal<String>) -> Self {
        Self::Reactive(value)
    }
}

/// Builder for a text node.
pub struct TextBuilder {
    source: TextContent,
    a11y: A11yProps,
    wrap: bool,
    ellipsis: bool,
    pre_extensions: Vec<TextMutator>,
}

/// Create a text node builder.
#[must_use]
pub fn text(source: impl Into<TextContent>) -> TextBuilder {
    TextBuilder {
        source: source.into(),
        a11y: A11yProps {
            role: Role::Label,
            ..A11yProps::default()
        },
        wrap: false,
        ellipsis: false,
        pre_extensions: Vec::new(),
    }
}

impl TextBuilder {
    /// Wrap the text to its containing block's width instead of hugging
    /// its content on one line.
    ///
    /// Off by default: a bare label hugs its content and the container
    /// clips or scrolls any overflow. Turn it on for prose that should
    /// reflow — descriptions, chat bodies, help text — where the height
    /// grows with the content and the width follows the container.
    #[must_use]
    pub const fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Keep the text on one line and truncate it with a trailing ellipsis
    /// (`…`) when it is given less width than it needs (CSS `text-overflow:
    /// ellipsis`).
    ///
    /// Off by default (a bare label hugs its content and overflows). Turn
    /// it on for single-line controls in a width-constrained slot — a file
    /// name in a fixed column, a title in a card — so a long value reads as
    /// "Some long va…" instead of overflowing or clipping mid-glyph. Takes
    /// precedence over [`Self::wrap`]; a node should set one or the other.
    #[must_use]
    pub const fn ellipsize(mut self) -> Self {
        self.ellipsis = true;
        self
    }

    /// Set the accessibility role.
    #[must_use]
    pub const fn role(mut self, role: Role) -> Self {
        self.a11y.role = role;
        self
    }

    /// Set the accessible label.
    ///
    /// Accepts a `&'static str` / `String` (static) or `Signal<String>`
    /// (reactive) via the `AccessibleName` `From` impls.
    #[must_use]
    pub fn label(mut self, name: impl Into<AccessibleName>) -> Self {
        self.a11y.name = name.into();
        self
    }

    /// Set a static accessible description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.a11y.description = Some(AccessibleName::Static(description.into()));
        self
    }

    /// Mark this text node focusable or not.
    #[must_use]
    pub const fn focusable(mut self, yes: bool) -> Self {
        self.a11y.focusable = yes;
        self
    }

    /// Set an explicit tab index.
    #[must_use]
    pub const fn tab_index(mut self, index: i32) -> Self {
        self.a11y.tab_index = Some(index);
        self
    }

    /// Set the keyboard shortcut advertised by this text node.
    #[must_use]
    pub fn keyboard_shortcut(mut self, shortcut: impl Into<Shortcut>) -> Self {
        self.a11y.keyboard_shortcut = Some(shortcut.into());
        self
    }

    /// Enable or disable pointer hit-testing for this text node.
    #[must_use]
    pub const fn hittable(mut self, yes: bool) -> Self {
        self.a11y.hittable = yes;
        self
    }

    /// Add this builder to a context.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }

    /// Mutate the node at build time.
    ///
    /// This is the escape hatch used by downstream builder extension
    /// traits that need to attach typed extensions to text nodes.
    #[must_use]
    pub fn mutate(mut self, apply: impl FnOnce(&mut crate::Node) + 'static) -> Self {
        self.pre_extensions.push(Box::new(apply));
        self
    }
}

impl IntoNode for TextBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        let id = cx.new_node(
            NodeType::Text {
                content: self.source,
                wrap: self.wrap,
                ellipsis: self.ellipsis,
            },
            self.a11y,
        );
        if let Some(node) = cx.get_mut(id) {
            for apply in self.pre_extensions {
                apply(node);
            }
        }
        id
    }
}
