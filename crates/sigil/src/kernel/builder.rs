use crate::kernel::{Action, SortDirection};
use crate::reactive::Signal;
use smallvec::SmallVec;

use crate::kernel::component::{ErasedComponent, IntoNode, NodePatch};
use crate::kernel::{A11yProps, Cx, Node, NodeId, NodeType, TextCursor, reactive_children};
use crate::style::{Paint, StateStyleBuilder, States, Style};
use crate::widgets::Wiring;

mod keyed;
mod patch;

pub(crate) use patch::Fate;

type NodeMutator = Box<dyn FnOnce(&mut Node)>;
type ReactiveChildrenInstall = Box<dyn FnOnce(NodeId)>;

type AddedHook = Box<dyn FnOnce(&mut Cx, NodeId)>;

#[doc = include_str!("../../docs/node-builder.md")]
#[must_use]
pub struct NodeBuilder {
    pub(super) a11y: A11yProps,
    pub(super) children: SmallVec<[Box<dyn ErasedComponent>; 4]>,
    pub(super) kind: NodeType,
    pub(super) pre_extensions: Vec<NodeMutator>,
    pub(super) reactive_children: Option<ReactiveChildrenInstall>,
    pub(super) added: Vec<AddedHook>,
    style: Option<Box<Style>>,
    pub(super) wraps: Option<Box<dyn ErasedComponent>>,
    wiring: Option<Box<Wiring>>,
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
pub fn container() -> NodeBuilder {
    NodeBuilder::new(NodeType::Container)
}

/// Children without a box of their own.
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
            added: Vec::new(),
            style: None,
            wraps: None,
            wiring: None,
        }
    }

    pub(crate) fn wrapping(widget: impl IntoNode + 'static) -> Self {
        Self {
            wraps: Some(Box::new(widget)),
            ..Self::new(NodeType::Container)
        }
    }

    shared_node_methods!();

    /// Names `node` as the node that labels this node for assistive technology.
    pub fn labelled_by(mut self, node: NodeId) -> Self {
        self.a11y.labelled_by = Some(node);
        self
    }

    /// Adds `action` to the accessibility actions of the node.
    pub fn action(mut self, action: Action) -> Self {
        self.a11y.actions.push(action);
        self
    }

    /// Sets whether a Tab press goes to the focused node instead of moving focus to the next node.
    pub const fn traps_tab(mut self, yes: bool) -> Self {
        self.a11y.traps_tab = yes;
        self
    }

    /// Sets the signal that reports whether the node is checked.
    pub fn checked(mut self, signal: Signal<bool>) -> Self {
        self.a11y.checked = Some(signal);
        self
    }

    /// Sets the signal that announces the node as mixed, neither checked nor unchecked.
    pub fn mixed(mut self, signal: Signal<bool>) -> Self {
        self.a11y.mixed = Some(signal);
        self
    }

    /// Sets the signal that reports whether the node is selected.
    pub fn selected(mut self, signal: Signal<bool>) -> Self {
        self.a11y.selected = Some(signal);
        self
    }

    /// Sets the signal that reports whether the node is expanded.
    pub fn expanded(mut self, signal: Signal<bool>) -> Self {
        self.a11y.expanded = Some(signal);
        self
    }

    /// Sets the signal that reports the sort direction; `None` in the signal means not sorted.
    pub fn sort_direction(mut self, signal: Signal<Option<SortDirection>>) -> Self {
        self.a11y.sort_direction = Some(signal);
        self
    }

    /// Sets the signal that reports the number of rows.
    pub fn row_count(mut self, signal: Signal<usize>) -> Self {
        self.a11y.row_count = Some(signal);
        self
    }

    /// Sets the number of columns that the node reports.
    pub fn column_count(mut self, count: usize) -> Self {
        self.a11y.column_count = Some(count);
        self
    }

    /// Sets the row index that the node reports.
    pub fn row_index(mut self, index: usize) -> Self {
        self.a11y.row_index = Some(index);
        self
    }

    /// Sets the column index that the node reports.
    pub fn column_index(mut self, index: usize) -> Self {
        self.a11y.column_index = Some(index);
        self
    }

    /// Sets the numeric value signal and its range; a reversed `min` and `max` swap.
    pub fn value(mut self, signal: Signal<f32>, min: f32, max: f32) -> Self {
        let (low, high) = (min.min(max), min.max(max));
        let valid = !low.is_nan();
        self.a11y.value_now = Some(signal);
        self.a11y.value_min = valid.then_some(low);
        self.a11y.value_max = valid.then_some(high);
        self
    }

    /// Sets the step of the numeric value that the node reports to assistive technology.
    pub fn value_step(mut self, step: f32) -> Self {
        self.a11y.step = Some(step);
        self
    }

    /// Sets the signal that holds the text value of the node.
    pub fn text_value(mut self, signal: Signal<String>) -> Self {
        self.a11y.text_value = Some(signal);
        self
    }

    /// Sets the signals for the caret position and the selection anchor of the text.
    pub fn text_cursor(mut self, caret: Signal<usize>, anchor: Signal<Option<usize>>) -> Self {
        self.a11y.text_cursor = Some(TextCursor { caret, anchor });
        self
    }

    /// Sets the placeholder text that the node reports to assistive technology.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.a11y.placeholder = Some(placeholder.into());
        self
    }

    /// Adds a child.
    pub fn child(mut self, child: impl IntoNode + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Appends every child in `children`.
    pub fn children<I, N>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = N>,
        N: IntoNode + 'static,
    {
        for child in children {
            self.children.push(Box::new(child));
        }
        self
    }

    /// Runs `hook` with the tree and the node id after the node and its children are in the tree.
    pub fn added(mut self, hook: impl FnOnce(&mut Cx, NodeId) + 'static) -> Self {
        self.added.push(Box::new(hook));
        self
    }

    /// Builds the children from `f` and rebuilds them when a signal that `f` reads changes.
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

    /// Builds the children from the `(key, child)` pairs of `f`; a key names a child across runs.
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

    /// Attaches `value` to the node as an extension that `Node::extension` reads.
    pub fn with<T: 'static>(mut self, value: T) -> Self {
        self.pre_extensions.push(Box::new(move |node: &mut Node| {
            let _ = node.attach(value);
        }));
        self
    }

    pub(crate) fn pending_style(&mut self) -> &mut Style {
        self.style.get_or_insert_with(|| Box::new(Style::new()))
    }

    pub(crate) const fn take_style(&mut self) -> Option<Box<Style>> {
        self.style.take()
    }

    fn styled(mut self) -> Self {
        let style = self.style.take().filter(|style| !style.is_empty());
        match style {
            Some(style) => crate::style::attach_style(self, *style),
            None if self.wiring.as_ref().is_none_or(|wiring| wiring.is_inert())
                && !self.is_focusable() =>
            {
                self
            }
            None => crate::style::attach_style(self, Style::new()),
        }
    }

    pub(crate) fn wiring_mut(&mut self) -> &mut Wiring {
        self.wiring.get_or_insert_default()
    }

    pub(crate) fn take_wiring(&mut self) -> Wiring {
        self.wiring.take().map(|wiring| *wiring).unwrap_or_default()
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

impl Paint for NodeBuilder {
    fn paint_mut(&mut self) -> &mut StateStyleBuilder {
        &mut self.pending_style().paint
    }
}

impl States for NodeBuilder {
    crate::style::state_methods!(style pending_style());
}

impl IntoNode for NodeBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        let mut this = self.styled();
        if let Some(widget) = this.wraps.take() {
            return this.build_onto(widget, cx);
        }
        let kind = std::mem::replace(&mut this.kind, NodeType::Container).into_live();
        let id = cx.new_node(kind, std::mem::take(&mut this.a11y));
        crate::kernel::text::track_delta(cx, id);
        this.finish(cx, id)
    }

    fn patch(self, cx: &mut Cx, existing: NodeId) -> NodePatch<Self> {
        self.patch_onto(cx, existing)
    }
}
