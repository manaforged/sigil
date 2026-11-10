use crate::events::Shortcut;
use crate::kernel::{Action, Role, SortDirection};
use crate::reactive::Signal;
use smallvec::SmallVec;

use crate::kernel::component::{ErasedComponent, IntoNode, Patch};
use crate::kernel::{
    A11yProps, AccessibleName, Cx, Node, NodeId, NodeType, TextCursor, reactive_children,
};
use crate::style::{PaintTarget, Style, StyleSlot};
use crate::widgets::Wiring;

mod keyed;
mod patch;

pub(crate) use patch::Fate;

type NodeMutator = Box<dyn FnOnce(&mut Node)>;
type ReactiveChildrenInstall = Box<dyn FnOnce(NodeId)>;

type AddedHook = Box<dyn FnOnce(&mut Cx, NodeId)>;

#[doc = include_str!("../../docs/node-builder.md")]
pub struct NodeBuilder {
    pub(super) a11y: A11yProps,
    pub(super) children: SmallVec<[Box<dyn ErasedComponent>; 4]>,
    pub(super) kind: NodeType,
    pub(super) pre_extensions: Vec<NodeMutator>,
    pub(super) reactive_children: Option<ReactiveChildrenInstall>,
    pub(super) on_added: Vec<AddedHook>,
    style: Style,
    pub(super) wraps: Option<Box<dyn ErasedComponent>>,
    wiring: Wiring,
}

impl core::fmt::Debug for NodeBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NodeBuilder")
            .field("a11y", &self.a11y)
            .field("kind", &self.kind)
            .field("style", &self.style)
            .finish_non_exhaustive()
    }
}

/// A box that holds children.
#[must_use]
pub fn container() -> NodeBuilder {
    NodeBuilder::new(NodeType::Container)
}

/// Children without a box of their own.
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
            wiring: Wiring::default(),
        }
    }

    pub(crate) fn wrapping(widget: impl IntoNode + 'static) -> Self {
        Self {
            wraps: Some(Box::new(widget)),
            ..Self::new(NodeType::Container)
        }
    }

    /// Sets the role.
    #[must_use]
    pub const fn role(mut self, role: Role) -> Self {
        self.a11y.role = role;
        self
    }

    /// Sets the label.
    #[must_use]
    pub fn label(mut self, name: impl Into<AccessibleName>) -> Self {
        self.a11y.name = name.into();
        self
    }

    /// Sets the description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.a11y.description = Some(AccessibleName::Static(description.into()));
        self
    }

    /// Sets the labelled by.
    #[must_use]
    pub const fn labelled_by(mut self, node: NodeId) -> Self {
        self.a11y.labelled_by = Some(node);
        self
    }

    /// Sets the owns.
    #[must_use]
    pub fn owns(mut self, node: NodeId) -> Self {
        self.a11y.owns.push(node);
        self
    }

    /// Sets the action.
    #[must_use]
    pub fn action(mut self, action: Action) -> Self {
        self.a11y.actions.push(action);
        self
    }

    /// Sets the focusable.
    #[must_use]
    pub fn focusable(mut self, yes: bool) -> Self {
        debug_assert!(
            !(yes && matches!(self.kind, NodeType::Fragment)),
            "focusable(true) on a Fragment: fragments have no geometry"
        );
        self.a11y.focusable = yes;
        self
    }

    /// Sets the traps tab.
    #[must_use]
    pub const fn traps_tab(mut self, yes: bool) -> Self {
        self.a11y.traps_tab = yes;
        self
    }

    /// Sets the tab index.
    #[must_use]
    pub const fn tab_index(mut self, index: i32) -> Self {
        self.a11y.tab_index = Some(index);
        self
    }

    /// Sets the keyboard shortcut.
    #[must_use]
    pub fn keyboard_shortcut(mut self, shortcut: impl Into<Shortcut>) -> Self {
        self.a11y.keyboard_shortcut = Some(shortcut.into());
        self
    }

    /// Sets the hittable.
    #[must_use]
    pub const fn hittable(mut self, yes: bool) -> Self {
        self.a11y.hittable = yes;
        self
    }

    /// Sets the checked.
    #[must_use]
    pub const fn checked(mut self, signal: Signal<bool>) -> Self {
        self.a11y.checked = Some(signal);
        self
    }

    /// Sets the signal that announces the node as mixed, neither checked nor unchecked.
    #[must_use]
    pub const fn mixed(mut self, signal: Signal<bool>) -> Self {
        self.a11y.mixed = Some(signal);
        self
    }

    /// Sets the selected.
    #[must_use]
    pub const fn selected(mut self, signal: Signal<bool>) -> Self {
        self.a11y.selected = Some(signal);
        self
    }

    /// Sets the expanded.
    #[must_use]
    pub const fn expanded(mut self, signal: Signal<bool>) -> Self {
        self.a11y.expanded = Some(signal);
        self
    }

    /// Sets the sort direction.
    #[must_use]
    pub const fn sort_direction(mut self, signal: Signal<Option<SortDirection>>) -> Self {
        self.a11y.sort_direction = Some(signal);
        self
    }

    /// Sets the row count.
    #[must_use]
    pub const fn row_count(mut self, signal: Signal<usize>) -> Self {
        self.a11y.row_count = Some(signal);
        self
    }

    /// Sets the column count.
    #[must_use]
    pub const fn column_count(mut self, count: usize) -> Self {
        self.a11y.column_count = Some(count);
        self
    }

    /// Sets the row index.
    #[must_use]
    pub const fn row_index(mut self, index: usize) -> Self {
        self.a11y.row_index = Some(index);
        self
    }

    /// Sets the column index.
    #[must_use]
    pub const fn column_index(mut self, index: usize) -> Self {
        self.a11y.column_index = Some(index);
        self
    }

    /// Sets the value.
    #[must_use]
    pub fn value(mut self, signal: Signal<f32>, min: f32, max: f32) -> Self {
        let (low, high) = (min.min(max), min.max(max));
        let valid = !low.is_nan();
        self.a11y.value_now = Some(signal);
        self.a11y.value_min = valid.then_some(low);
        self.a11y.value_max = valid.then_some(high);
        self
    }

    /// Sets the value step.
    #[must_use]
    pub const fn value_step(mut self, step: f32) -> Self {
        self.a11y.step = Some(step);
        self
    }

    /// Sets the text value.
    #[must_use]
    pub const fn text_value(mut self, signal: Signal<String>) -> Self {
        self.a11y.text_value = Some(signal);
        self
    }

    /// Sets the text cursor.
    #[must_use]
    pub const fn text_cursor(
        mut self,
        caret: Signal<usize>,
        anchor: Signal<Option<usize>>,
    ) -> Self {
        self.a11y.text_cursor = Some(TextCursor { caret, anchor });
        self
    }

    /// Sets the placeholder.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.a11y.placeholder = Some(placeholder.into());
        self
    }

    /// Adds a child.
    #[must_use]
    pub fn child(mut self, child: impl IntoNode + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Sets the callback that runs on added.
    #[must_use]
    pub fn on_added(mut self, hook: impl FnOnce(&mut Cx, NodeId) + 'static) -> Self {
        self.on_added.push(Box::new(hook));
        self
    }

    /// Sets the children reactive.
    #[must_use]
    #[track_caller]
    pub fn children_reactive<F, I, N>(mut self, f: F) -> Self
    where
        F: Fn() -> I + 'static,
        I: IntoIterator<Item = N>,
        N: IntoNode + 'static,
    {
        let location = core::panic::Location::caller();
        self.reactive_children = Some(Box::new(move |parent| {
            reactive_children::install(parent, location, f);
        }));
        self
    }

    /// Sets the children keyed.
    #[must_use]
    #[track_caller]
    pub fn children_keyed<F, I, K, N>(self, f: F) -> Self
    where
        F: Fn() -> I + 'static,
        I: IntoIterator<Item = (K, N)>,
        K: Clone + Eq + std::hash::Hash + 'static,
        N: IntoNode + 'static,
    {
        self.keyed_children(f)
    }

    /// Sets the with.
    #[must_use]
    pub fn with<T: 'static>(mut self, value: T) -> Self {
        self.pre_extensions.push(Box::new(move |node: &mut Node| {
            let _ = node.attach(value);
        }));
        self
    }

    /// Sets the mutate.
    #[must_use]
    pub fn mutate(mut self, apply: impl FnOnce(&mut Node) + 'static) -> Self {
        self.pre_extensions.push(Box::new(apply));
        self
    }

    /// Adds this node to the tree and returns its id.
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

    pub(crate) const fn wiring_mut(&mut self) -> &mut Wiring {
        &mut self.wiring
    }

    pub(crate) const fn is_focusable(&self) -> bool {
        self.a11y.focusable
    }

    pub(crate) fn map_keeping_kind(mut self, f: impl FnOnce(Self) -> Self) -> Self {
        let kind = std::mem::replace(&mut self.kind, NodeType::Container);
        let mut node = f(self);
        node.kind = kind;
        node
    }
}

impl PaintTarget for NodeBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style)
    }
}

impl IntoNode for NodeBuilder {
    fn build(mut self, cx: &mut Cx) -> NodeId {
        let style = std::mem::take(&mut self.style);
        let mut this = crate::style::attach_style(self, style);
        if let Some(widget) = this.wraps.take() {
            return this.build_onto(widget, cx);
        }
        let kind = std::mem::replace(&mut this.kind, NodeType::Container).into_live();
        let id = cx.new_node(kind, std::mem::take(&mut this.a11y));
        crate::kernel::text::track_delta(cx, id);
        this.finish(cx, id)
    }

    fn patch(self, cx: &mut Cx, existing: NodeId) -> Patch<Self> {
        self.patch_onto(cx, existing)
    }
}
