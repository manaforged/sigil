//! Generic builders for structural nodes.

use accesskit::{Action, Role};
use hal::Shortcut;
use reactive::Signal;
use smallvec::SmallVec;

use crate::component::{ErasedComponent, IntoNode};
use crate::{A11yProps, AccessibleName, Cx, Node, NodeId, NodeType, reactive_children};

type NodeMutator = Box<dyn FnOnce(&mut Node)>;
type ReactiveChildrenInstall = Box<dyn FnOnce(NodeId)>;

/// Builder for structural nodes.
pub struct NodeBuilder {
    a11y: A11yProps,
    children: SmallVec<[Box<dyn ErasedComponent>; 4]>,
    kind: NodeType,
    pre_extensions: Vec<NodeMutator>,
    reactive_children: Option<ReactiveChildrenInstall>,
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

    /// Set a reactive numeric value for range-like controls.
    #[must_use]
    pub const fn value(mut self, signal: Signal<f32>, min: f32, max: f32) -> Self {
        self.a11y.value_now = Some(signal);
        self.a11y.value_min = Some(min);
        self.a11y.value_max = Some(max);
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
    /// Until B1.b wires the drain into `LayoutEngine::compute`, callers
    /// must invoke [`Cx::flush_reactive_children`] explicitly after
    /// signal mutations (and after the initial `cx.add(...)`) to apply
    /// the queued rebuild.
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

    /// Attach a typed extension value to this node at build time.
    ///
    /// This is the low-level primitive. Prefer crate-provided extension
    /// methods — for example, use `.layout(LayoutStyle::…)` from
    /// `layout::prelude::*` rather than `.with(layout_style)`.
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
        id
    }
}
