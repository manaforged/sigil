use crate::events::Shortcut;
use crate::reactive::Signal;
use accesskit::{Action, Role, SortDirection};
use smallvec::SmallVec;

use crate::kernel::component::{ErasedComponent, IntoNode};
use crate::kernel::{
    A11yProps, AccessibleName, Cx, Node, NodeId, NodeType, TextCursor, reactive_children,
};
use crate::style::{PaintTarget, StateHost, Style, StyleSlot};

type NodeMutator = Box<dyn FnOnce(&mut Node)>;
type ReactiveChildrenInstall = Box<dyn FnOnce(NodeId)>;

type AddedHook = Box<dyn FnOnce(&mut Cx, NodeId)>;

pub struct NodeBuilder {
    pub(super) a11y: A11yProps,
    pub(super) children: SmallVec<[Box<dyn ErasedComponent>; 4]>,
    kind: NodeType,
    pub(super) pre_extensions: Vec<NodeMutator>,
    pub(super) reactive_children: Option<ReactiveChildrenInstall>,
    pub(super) on_added: Vec<AddedHook>,
    style: Style,
    pub(super) wraps: Option<Box<dyn ErasedComponent>>,
}

#[must_use]
pub fn container() -> NodeBuilder {
    NodeBuilder::new(NodeType::Container)
}

#[must_use]
pub fn fragment() -> NodeBuilder {
    NodeBuilder::new(NodeType::Fragment)
}

impl NodeBuilder {
    pub(crate) fn from_parts(
        kind: NodeType,
        a11y: A11yProps,
        pre_extensions: Vec<NodeMutator>,
    ) -> Self {
        Self {
            a11y,
            pre_extensions,
            ..Self::new(kind)
        }
    }

    fn new(kind: NodeType) -> Self {
        Self {
            a11y: A11yProps::default(),
            children: SmallVec::new(),
            kind,
            pre_extensions: Vec::new(),
            reactive_children: None,
            on_added: Vec::new(),
            style: Style::new(),
            wraps: None,
        }
    }

    pub(crate) fn wrapping(widget: impl IntoNode + 'static) -> Self {
        Self {
            wraps: Some(Box::new(widget)),
            ..Self::new(NodeType::Container)
        }
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
    pub const fn labelled_by(mut self, node: NodeId) -> Self {
        self.a11y.labelled_by = Some(node);
        self
    }

    #[must_use]
    pub fn owns(mut self, node: NodeId) -> Self {
        self.a11y.owns.push(node);
        self
    }

    #[must_use]
    pub fn action(mut self, action: Action) -> Self {
        self.a11y.actions.push(action);
        self
    }

    #[must_use]
    pub fn focusable(mut self, yes: bool) -> Self {
        debug_assert!(
            !(yes && matches!(self.kind, NodeType::Fragment)),
            "focusable(true) on a Fragment: fragments have no geometry"
        );
        self.a11y.focusable = yes;
        self
    }

    #[must_use]
    pub const fn traps_tab(mut self, yes: bool) -> Self {
        self.a11y.traps_tab = yes;
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

    #[must_use]
    pub const fn checked(mut self, signal: Signal<bool>) -> Self {
        self.a11y.checked = Some(signal);
        self
    }

    #[must_use]
    pub const fn selected(mut self, signal: Signal<bool>) -> Self {
        self.a11y.selected = Some(signal);
        self
    }

    #[must_use]
    pub const fn expanded(mut self, signal: Signal<bool>) -> Self {
        self.a11y.expanded = Some(signal);
        self
    }

    #[must_use]
    pub const fn sort_direction(mut self, signal: Signal<Option<SortDirection>>) -> Self {
        self.a11y.sort_direction = Some(signal);
        self
    }

    #[must_use]
    pub const fn row_count(mut self, signal: Signal<usize>) -> Self {
        self.a11y.row_count = Some(signal);
        self
    }

    #[must_use]
    pub const fn column_count(mut self, count: usize) -> Self {
        self.a11y.column_count = Some(count);
        self
    }

    #[must_use]
    pub const fn row_index(mut self, index: usize) -> Self {
        self.a11y.row_index = Some(index);
        self
    }

    #[must_use]
    pub const fn column_index(mut self, index: usize) -> Self {
        self.a11y.column_index = Some(index);
        self
    }

    #[must_use]
    pub fn value(mut self, signal: Signal<f32>, min: f32, max: f32) -> Self {
        let (low, high) = (min.min(max), min.max(max));
        let valid = !low.is_nan();
        self.a11y.value_now = Some(signal);
        self.a11y.value_min = valid.then_some(low);
        self.a11y.value_max = valid.then_some(high);
        self
    }

    #[must_use]
    pub const fn value_step(mut self, step: f32) -> Self {
        self.a11y.step = Some(step);
        self
    }

    #[must_use]
    pub const fn text_value(mut self, signal: Signal<String>) -> Self {
        self.a11y.text_value = Some(signal);
        self
    }

    #[must_use]
    pub const fn text_cursor(
        mut self,
        caret: Signal<usize>,
        anchor: Signal<Option<usize>>,
    ) -> Self {
        self.a11y.text_cursor = Some(TextCursor { caret, anchor });
        self
    }

    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.a11y.placeholder = Some(placeholder.into());
        self
    }

    #[must_use]
    pub fn child(mut self, child: impl IntoNode + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    #[must_use]
    pub fn on_added(mut self, hook: impl FnOnce(&mut Cx, NodeId) + 'static) -> Self {
        self.on_added.push(Box::new(hook));
        self
    }

    #[must_use]
    pub fn children_reactive<F, I, N>(mut self, f: F) -> Self
    where
        F: Fn() -> I + 'static,
        I: IntoIterator<Item = N>,
        N: IntoNode + 'static,
    {
        self.reactive_children = Some(Box::new(move |parent| {
            reactive_children::install(parent, f);
        }));
        self
    }

    #[must_use]
    pub fn children_keyed<F, I, K, N>(mut self, f: F) -> Self
    where
        F: Fn() -> I + 'static,
        I: IntoIterator<Item = (K, N)>,
        K: Clone + Eq + std::hash::Hash + 'static,
        N: IntoNode + 'static,
    {
        self.reactive_children = Some(Box::new(move |parent| {
            reactive_children::install_keyed(parent, f);
        }));
        self
    }

    #[must_use]
    pub fn with<T: 'static>(mut self, value: T) -> Self {
        self.pre_extensions.push(Box::new(move |node: &mut Node| {
            let _ = node.attach(value);
        }));
        self
    }

    #[must_use]
    pub fn mutate(mut self, apply: impl FnOnce(&mut Node) + 'static) -> Self {
        self.pre_extensions.push(Box::new(apply));
        self
    }

    #[allow(
        clippy::should_implement_trait,
        reason = "intentional, reviewed trade-off"
    )]
    pub fn add(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }

    pub(crate) const fn pending_style(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl PaintTarget for NodeBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style)
    }
}

impl StateHost for NodeBuilder {}

impl IntoNode for NodeBuilder {
    fn build(mut self, cx: &mut Cx) -> NodeId {
        let style = std::mem::take(&mut self.style);
        let mut this = crate::style::attach_style(self, style);
        if let Some(widget) = this.wraps.take() {
            return this.build_onto(widget, cx);
        }
        let kind = std::mem::replace(&mut this.kind, NodeType::Container);
        let id = cx.new_node(kind, std::mem::take(&mut this.a11y));
        this.finish(cx, id)
    }
}
