//! Generic builders for structural nodes.

use accesskit::{Action, Role};
use sigil_reactive::Signal;
use smallvec::SmallVec;

use crate::component::{BoxedComponent, Component};
use crate::{Cx, NameSource, NodeId, NodeKind, Semantics};

/// Builder for structural nodes.
pub struct NodeBuilder {
    semantics: Semantics,
    children: SmallVec<[Box<dyn BoxedComponent>; 4]>,
    kind: NodeKind,
}

/// Create a container node builder.
#[must_use]
pub fn container() -> NodeBuilder {
    NodeBuilder::new(NodeKind::Container)
}

/// Create a fragment node builder.
#[must_use]
pub fn fragment() -> NodeBuilder {
    NodeBuilder::new(NodeKind::Fragment)
}

impl NodeBuilder {
    fn new(kind: NodeKind) -> Self {
        Self {
            semantics: Semantics::default(),
            children: SmallVec::new(),
            kind,
        }
    }

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

    /// Set this node's labelling relation.
    #[must_use]
    pub fn labelled_by(mut self, node: NodeId) -> Self {
        self.semantics.aria_labelled_by = Some(node);
        self.semantics.name = NameSource::LabelledBy(node);
        self
    }

    /// Add an owned accessibility relation.
    #[must_use]
    pub fn owns(mut self, node: NodeId) -> Self {
        self.semantics.aria_owns.push(node);
        self
    }

    /// Add a supported platform action.
    #[must_use]
    pub fn action(mut self, action: Action) -> Self {
        self.semantics.actions.push(action);
        self
    }

    /// Mark this node focusable or not.
    #[must_use]
    pub const fn focusable(mut self, yes: bool) -> Self {
        self.semantics.focusable = yes;
        self
    }

    /// Set an explicit tab index.
    #[must_use]
    pub const fn tab_index(mut self, index: i32) -> Self {
        self.semantics.tab_index = Some(index);
        self
    }

    /// Add a child component to this structural node.
    #[must_use]
    pub fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Add this builder to a context.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }
}

impl Component for NodeBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        let id = cx.insert_raw(self.kind, self.semantics);
        for child in self.children {
            let _ = cx.add_boxed_child(id, child);
        }
        id
    }
}
