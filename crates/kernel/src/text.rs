//! Built-in text node primitive.

use std::fmt;
use std::rc::Rc;

use accesskit::Role;
use sigil_hal::Shortcut;
use sigil_reactive::{Signal, derived};

use crate::{A11yProps, AccessibleName, Cx, IntoNode, NodeId, NodeType};

type TextMutator = Box<dyn FnOnce(&mut crate::Node)>;

/// Static or live text content.
#[derive(Clone)]
pub enum TextContent {
    /// Fixed text known at build time.
    Static(String),
    /// Text driven by a reactive signal.
    Reactive(Signal<String>),
    /// Factory-safe live text. [`TextBuilder`] converts this to
    /// [`TextContent::Reactive`] under the node owner so a keyed
    /// keep does not dispose the subscription.
    Compute(Rc<dyn Fn() -> String>),
}

impl TextContent {
    /// Borrow the static text when this source is fixed.
    #[must_use]
    pub fn as_static_str(&self) -> Option<&str> {
        match self {
            Self::Static(value) => Some(value),
            Self::Reactive(_) | Self::Compute(_) => None,
        }
    }

    /// Resolve the current string.
    #[must_use]
    pub fn resolve(&self) -> String {
        match self {
            Self::Static(value) => value.clone(),
            Self::Reactive(signal) => signal.try_peek().unwrap_or_default(),
            Self::Compute(compute) => compute(),
        }
    }

    /// Resolve into `out`, reporting whether the value changed.
    ///
    /// Fixed text compares in place and allocates nothing. A signal or a
    /// closure has to produce its `String` to be compared at all, so this
    /// produces it once — asking "did it change" and then "what is it" would
    /// produce it twice.
    pub fn refresh(&self, out: &mut String) -> bool {
        match self {
            Self::Static(value) => replace(out, value),
            Self::Reactive(signal) => match signal.try_peek() {
                Some(value) => replace(out, &value),
                None => replace(out, ""),
            },
            Self::Compute(compute) => replace(out, &compute()),
        }
    }

    /// Resolve into `out`, reusing its buffer.
    ///
    /// Fixed text copies into the buffer and allocates nothing once it is
    /// large enough. A painter resolving every text node each frame keeps one
    /// buffer per node instead of a fresh `String` per node per frame.
    pub fn resolve_into(&self, out: &mut String) {
        out.clear();
        match self {
            Self::Static(value) => out.push_str(value),
            Self::Reactive(signal) => {
                if let Some(value) = signal.try_peek() {
                    out.push_str(&value);
                }
            }
            Self::Compute(compute) => out.push_str(&compute()),
        }
    }

    /// Live text from a closure. The closure is stored until
    /// [`TextBuilder`] builds the node, then wrapped in
    /// `sigil_reactive::derived`.
    #[must_use]
    pub fn compute(f: impl Fn() -> String + 'static) -> Self {
        Self::Compute(Rc::new(f))
    }

    /// Convert factory-safe compute text into a node-owned signal.
    fn into_live(self) -> Self {
        match self {
            Self::Compute(compute) => Self::Reactive(derived(move || compute())),
            other => other,
        }
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

/// Create a live text node from a closure.
///
/// The closure becomes a `sigil_reactive::derived` at build time, under
/// the node's owner, so a keyed keep does not dispose the subscription.
/// Use this in factories that run before the node owner exists.
#[must_use]
pub fn text_live(f: impl Fn() -> String + 'static) -> TextBuilder {
    text(TextContent::compute(f))
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
        wrap: true,
        ellipsis: false,
        pre_extensions: Vec::new(),
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
        let id = cx.new_node(
            NodeType::Text {
                content: self.source.into_live(),
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

/// Overwrite `out` with `value`, reporting whether it differed.
fn replace(out: &mut String, value: &str) -> bool {
    if out == value {
        return false;
    }
    out.clear();
    out.push_str(value);
    true
}
