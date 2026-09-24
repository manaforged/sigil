use std::fmt;
use std::rc::Rc;

use crate::hal::Shortcut;
use crate::reactive::{Signal, derived};
use accesskit::Role;

use crate::kernel::{A11yProps, AccessibleName, Cx, IntoNode, NodeId, NodeType};

type TextMutator = Box<dyn FnOnce(&mut crate::kernel::Node)>;

#[derive(Clone)]
pub enum TextContent {
    Static(String),
    Reactive(Signal<String>),
    Compute(Rc<dyn Fn() -> String>),
}

impl TextContent {
    #[must_use]
    pub fn as_static_str(&self) -> Option<&str> {
        match self {
            Self::Static(value) => Some(value),
            Self::Reactive(_) | Self::Compute(_) => None,
        }
    }

    #[must_use]
    pub fn resolve(&self) -> String {
        match self {
            Self::Static(value) => value.clone(),
            Self::Reactive(signal) => signal.try_peek().unwrap_or_default(),
            Self::Compute(compute) => compute(),
        }
    }

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

    #[must_use]
    pub fn compute(f: impl Fn() -> String + 'static) -> Self {
        Self::Compute(Rc::new(f))
    }

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

pub struct TextBuilder {
    source: TextContent,
    a11y: A11yProps,
    wrap: bool,
    ellipsis: bool,
    pre_extensions: Vec<TextMutator>,
}

#[must_use]
pub fn text_live(f: impl Fn() -> String + 'static) -> TextBuilder {
    text(TextContent::compute(f))
}

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
    #[must_use]
    pub const fn wrap(mut self) -> Self {
        self.wrap = true;
        self.ellipsis = false;
        self
    }

    #[must_use]
    pub const fn nowrap(mut self) -> Self {
        self.wrap = false;
        self
    }

    #[must_use]
    pub const fn ellipsize(mut self) -> Self {
        self.ellipsis = true;
        self.wrap = false;
        self
    }

    #[must_use]
    pub const fn role(mut self, role: Role) -> Self {
        self.a11y.role = role;
        self
    }

    #[must_use]
    pub fn label(mut self, name: impl Into<AccessibleName>) -> Self {
        self.a11y.name = name.into();
        self
    }

    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.a11y.description = Some(AccessibleName::Static(description.into()));
        self
    }

    #[must_use]
    pub const fn focusable(mut self, yes: bool) -> Self {
        self.a11y.focusable = yes;
        self
    }

    #[must_use]
    pub const fn tab_index(mut self, index: i32) -> Self {
        self.a11y.tab_index = Some(index);
        self
    }

    #[must_use]
    pub fn keyboard_shortcut(mut self, shortcut: impl Into<Shortcut>) -> Self {
        self.a11y.keyboard_shortcut = Some(shortcut.into());
        self
    }

    #[must_use]
    pub const fn hittable(mut self, yes: bool) -> Self {
        self.a11y.hittable = yes;
        self
    }

    #[allow(
        clippy::should_implement_trait,
        reason = "intentional, reviewed trade-off"
    )]
    pub fn add(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }

    #[must_use]
    pub fn runs(self, runs: Vec<crate::style::TextRun>) -> Self {
        if runs.is_empty() {
            return self;
        }
        let runs = crate::style::TextRuns(runs.into());
        self.mutate(move |node| {
            node.attach(runs);
        })
    }

    #[must_use]
    pub fn mutate(mut self, apply: impl FnOnce(&mut crate::kernel::Node) + 'static) -> Self {
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

fn replace(out: &mut String, value: &str) -> bool {
    if out == value {
        return false;
    }
    out.clear();
    out.push_str(value);
    true
}
