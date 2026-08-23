//! Generic builders for structural nodes.

use accesskit::{Action, Role, SortDirection};
use sigil_hal::Shortcut;
use sigil_reactive::Signal;
use smallvec::SmallVec;

use crate::component::{ErasedComponent, IntoNode};
use crate::{A11yProps, AccessibleName, Cx, Node, NodeId, NodeType, reactive_children};

type NodeMutator = Box<dyn FnOnce(&mut Node)>;
type ReactiveChildrenInstall = Box<dyn FnOnce(NodeId)>;

/// One-shot hook run against the tree once the node has an id.
type AddedHook = Box<dyn FnOnce(&mut Cx, NodeId)>;

/// Builder for structural nodes (`container`, `fragment`).
///
/// Widget helpers implement [`IntoNode`]. Pass them to [`Self::child`].
/// Type a helper as `impl IntoNode`, not this type.
pub struct NodeBuilder {
    a11y: A11yProps,
    children: SmallVec<[Box<dyn ErasedComponent>; 4]>,
    kind: NodeType,
    pre_extensions: Vec<NodeMutator>,
    reactive_children: Option<ReactiveChildrenInstall>,
    on_added: Option<AddedHook>,
}

/// Create a container node builder.
#[must_use]
pub fn container() -> NodeBuilder {
    NodeBuilder::new(NodeType::Container)
}

/// Create a fragment node builder.
#[must_use]
pub fn fragment() -> NodeBuilder {
    NodeBuilder::new(NodeType::Fragment)
}

impl NodeBuilder {
    fn new(kind: NodeType) -> Self {
        Self {
            a11y: A11yProps::default(),
            children: SmallVec::new(),
            kind,
            pre_extensions: Vec::new(),
            reactive_children: None,
            on_added: None,
        }
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

    /// Set this node's labelling relation.
    #[must_use]
    pub const fn labelled_by(mut self, node: NodeId) -> Self {
        self.a11y.labelled_by = Some(node);
        self
    }

    /// Add an owned accessibility relation.
    #[must_use]
    pub fn owns(mut self, node: NodeId) -> Self {
        self.a11y.owns.push(node);
        self
    }

    /// Add a supported platform action.
    #[must_use]
    pub fn action(mut self, action: Action) -> Self {
        self.a11y.actions.push(action);
        self
    }

    /// Mark this node focusable or not.
    ///
    /// Fragments have no geometry; a focusable fragment enters the tab
    /// ring but has nothing for a focus indicator to draw against. See
    /// DR-015. Dev builds `debug_assert!` against the combination;
    /// release builds no-op.
    #[must_use]
    pub fn focusable(mut self, yes: bool) -> Self {
        debug_assert!(
            !(yes && matches!(self.kind, NodeType::Fragment)),
            "focusable(true) on a Fragment: fragments have no geometry; see DR-015"
        );
        self.a11y.focusable = yes;
        self
    }

    /// Trap a plain `Tab` key on this node when it is focused, so the node
    /// receives the key (e.g. to insert an indent) instead of the router moving
    /// focus. `Shift+Tab` still moves focus, so the node is never a keyboard trap.
    #[must_use]
    pub const fn traps_tab(mut self, yes: bool) -> Self {
        self.a11y.traps_tab = yes;
        self
    }

    /// Set an explicit tab index.
    #[must_use]
    pub const fn tab_index(mut self, index: i32) -> Self {
        self.a11y.tab_index = Some(index);
        self
    }

    /// Set the keyboard shortcut advertised by this node.
    #[must_use]
    pub fn keyboard_shortcut(mut self, shortcut: impl Into<Shortcut>) -> Self {
        self.a11y.keyboard_shortcut = Some(shortcut.into());
        self
    }

    /// Enable or disable pointer hit-testing for this node.
    #[must_use]
    pub const fn hittable(mut self, yes: bool) -> Self {
        self.a11y.hittable = yes;
        self
    }

    /// Set a reactive checked state for checkbox-like controls.
    #[must_use]
    pub const fn checked(mut self, signal: Signal<bool>) -> Self {
        self.a11y.checked = Some(signal);
        self
    }

    /// Set a reactive selected state for selectable controls.
    #[must_use]
    pub const fn selected(mut self, signal: Signal<bool>) -> Self {
        self.a11y.selected = Some(signal);
        self
    }

    /// Set the reactive sort direction for a grid column header.
    ///
    /// The signal yields `Some(direction)` while this column is the
    /// active sort and `None` otherwise.
    #[must_use]
    pub const fn sort_direction(mut self, signal: Signal<Option<SortDirection>>) -> Self {
        self.a11y.sort_direction = Some(signal);
        self
    }

    /// Set the reactive total row count for a grid (including header
    /// rows).
    #[must_use]
    pub const fn row_count(mut self, signal: Signal<usize>) -> Self {
        self.a11y.row_count = Some(signal);
        self
    }

    /// Set the total column count for a grid.
    #[must_use]
    pub const fn column_count(mut self, count: usize) -> Self {
        self.a11y.column_count = Some(count);
        self
    }

    /// Set the 1-based row index of this node within its grid.
    #[must_use]
    pub const fn row_index(mut self, index: usize) -> Self {
        self.a11y.row_index = Some(index);
        self
    }

    /// Set the 1-based column index of this cell within its row.
    #[must_use]
    pub const fn column_index(mut self, index: usize) -> Self {
        self.a11y.column_index = Some(index);
        self
    }

    /// Set a reactive numeric value for range-like controls.
    #[must_use]
    pub const fn value(mut self, signal: Signal<f32>, min: f32, max: f32) -> Self {
        self.a11y.value_now = Some(signal);
        self.a11y.value_min = Some(min);
        self.a11y.value_max = Some(max);
        self
    }

    /// Set the increment unit AT Increment/Decrement actions apply to
    /// this node's numeric value (defaults to 1.0 when unset).
    #[must_use]
    pub const fn value_step(mut self, step: f32) -> Self {
        self.a11y.step = Some(step);
        self
    }

    /// Set a reactive text value for text-input-like controls.
    #[must_use]
    pub const fn text_value(mut self, signal: Signal<String>) -> Self {
        self.a11y.text_value = Some(signal);
        self
    }

    /// Set placeholder text for text-input-like controls.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.a11y.placeholder = Some(placeholder.into());
        self
    }

    /// Add a child component to this structural node.
    #[must_use]
    pub fn child(mut self, child: impl IntoNode + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Run after this node and its static children exist in `cx`.
    #[must_use]
    pub fn on_added(mut self, hook: impl FnOnce(&mut Cx, NodeId) + 'static) -> Self {
        self.on_added = Some(Box::new(hook));
        self
    }

    /// Bind this node's children to a reactive closure.
    ///
    /// The closure runs once at build and re-runs whenever any signal
    /// it reads changes. Each re-run removes every prior child (full
    /// disposal of reactive owners) and adds the fresh result in source
    /// order. Identity across rebuilds is positional; callers that need
    /// per-row state must keep it in an external signal keyed by their
    /// own domain id.
    ///
    /// The bound effect is parented to this builder's per-component
    /// owner — removing the parent via [`Cx::remove`] disposes the
    /// subscription automatically.
    ///
    /// Layout compute drains the queue. Call
    /// [`Cx::flush_reactive_children`] only when you do not run layout.
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

    /// Bind children by key. A key that stays keeps its [`NodeId`].
    ///
    /// The closure returns `(key, child)` pairs. On each rebuild:
    /// matching keys keep the live node (the new child builder is
    /// dropped), new keys mount, missing keys dispose. Update label
    /// and style through signals on the kept node — a kept node is
    /// not rebuilt. Do not call `derived` in the factory; use
    /// [`crate::text_live`] so the derived is created in
    /// [`IntoNode::build`].
    ///
    /// First key wins when the list repeats a key. Use unique domain
    /// ids.
    ///
    /// Mix widget types with [`node`](crate::node).
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

    /// Attach a typed extension value to this node at build time.
    ///
    /// This is the low-level primitive. Prefer crate-provided extension
    /// methods — for example, use `.layout(LayoutStyle::…)` from
    /// `sigil_layout::prelude::*` rather than `.with(layout_style)`.
    #[must_use]
    pub fn with<T: 'static>(mut self, value: T) -> Self {
        self.pre_extensions.push(Box::new(move |node: &mut Node| {
            let _ = node.attach(value);
        }));
        self
    }

    /// Mutate the node at build time.
    ///
    /// This is the escape hatch used by downstream builder extension
    /// traits that need to merge into an existing typed extension
    /// instead of replacing it.
    #[must_use]
    pub fn mutate(mut self, apply: impl FnOnce(&mut Node) + 'static) -> Self {
        self.pre_extensions.push(Box::new(apply));
        self
    }

    /// Add this builder to a context.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }
}

impl IntoNode for NodeBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        let id = cx.new_node(self.kind, self.a11y);
        if let Some(node) = cx.get_mut(id) {
            for apply in self.pre_extensions {
                apply(node);
            }
        }
        for child in self.children {
            let _ = cx.add_boxed_child(id, child);
        }
        if let Some(install) = self.reactive_children {
            install(id);
        }
        if let Some(hook) = self.on_added {
            hook(cx, id);
        }
        id
    }
}
