//! Built-in text node primitive.

use std::fmt;
use std::rc::Rc;

use accesskit::Role;
use sigil_hal::Shortcut;
use sigil_reactive::{Signal, derived};

use crate::{A11yProps, AccessibleName, Cx, IntoNode, NodeId, NodeType};

type TextMutator = Box<dyn FnOnce(&mut crate::Node)>;

/// Static or live text content. Live nodes store only these two
/// forms; a factory closure (see [`text_live`]) becomes `Reactive`
/// at build time.
#[derive(Clone)]
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

    /// Resolve the current string.
    #[must_use]
    pub fn resolve(&self) -> String {
        match self {
            Self::Static(value) => value.clone(),
            Self::Reactive(signal) => signal.try_peek().unwrap_or_default(),
        }
    }
}

impl fmt::Debug for TextContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static(value) => f.debug_tuple("Static").field(value).finish(),
            Self::Reactive(_) => f.write_str("Reactive"),
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

/// What a [`TextBuilder`] builds from. A `Live` closure stays a
/// closure until build so the `derived` is created under the node's
/// owner, not the factory's effect-run owner.
enum Source {
    Content(TextContent),
    Live(Rc<dyn Fn() -> String>),
}

/// Builder for a text node.
pub struct TextBuilder {
    source: Source,
    a11y: A11yProps,
    wrap: bool,
    ellipsis: bool,
    pre_extensions: Vec<TextMutator>,
}

/// Create a text node builder.
#[must_use]
pub fn text(source: impl Into<TextContent>) -> TextBuilder {
    TextBuilder {
        source: Source::Content(source.into()),
        a11y: A11yProps {
            role: Role::Label,
            ..A11yProps::default()
        },
        wrap: true,
        ellipsis: false,
        pre_extensions: Vec::new(),
    }
}

/// Create a text node builder from a live closure.
///
/// The closure is stored on the builder and wrapped in
/// `sigil_reactive::derived` at build time, under the node's owner,
/// so a keyed keep does not dispose the subscription. Use this in
/// factories that run before the node owner exists.
#[must_use]
pub fn text_live(f: impl Fn() -> String + 'static) -> TextBuilder {
    TextBuilder {
        source: Source::Live(Rc::new(f)),
        ..text("")
    }
}

impl TextBuilder {
    /// Wrap to the containing block's width (the default).
    ///
    /// A definite-width parent reflows the paragraph. An indefinite parent
    /// still hugs max-content, so a toolbar label stays one line.
    #[must_use]
    pub const fn wrap(mut self) -> Self {
        self.wrap = true;
        self.ellipsis = false;
        self
    }

    /// Keep the text on one line and let a narrow parent overflow.
    ///
    /// Prefer [`Self::ellipsize`] when the slot has a definite width.
    #[must_use]
    pub const fn nowrap(mut self) -> Self {
        self.wrap = false;
        self
    }

    /// Keep the text on one line and truncate it with a trailing ellipsis
    /// (`…`) when it is given less width than it needs (CSS `text-overflow:
    /// ellipsis`).
    ///
    /// Use this for a single-line slot — a file name in a column, a title
    /// in a card — so a long value reads as "Some long va…" instead of
    /// wrapping. Clears [`Self::wrap`].
    #[must_use]
    pub const fn ellipsize(mut self) -> Self {
        self.ellipsis = true;
        self.wrap = false;
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
        let content = match self.source {
            Source::Content(content) => content,
            Source::Live(compute) => TextContent::Reactive(derived(move || compute())),
        };
        let id = cx.new_node(
            NodeType::Text {
                content,
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
