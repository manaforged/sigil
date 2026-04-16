//! Accessibility semantics carried by every node.

use sigil_reactive::Signal;
use smallvec::SmallVec;

use crate::NodeId;

/// Source for a node's accessible name or description.
#[derive(Clone, Debug)]
pub enum NameSource {
    /// No explicit source has been provided.
    None,
    /// A fixed string supplied by the component author.
    Static(String),
    /// A reactive string owned by the component's reactive owner.
    Derived(Signal<String>),
    /// The name is provided by another node in the same tree.
    LabelledBy(NodeId),
}

/// Accessibility metadata present on every node.
#[derive(Clone, Debug)]
pub struct Semantics {
    /// AccessKit role for this node.
    pub role: accesskit::Role,
    /// Accessible name source.
    pub name: NameSource,
    /// Optional accessible description source.
    pub description: Option<NameSource>,
    /// Supported platform actions.
    pub actions: SmallVec<[accesskit::Action; 4]>,
    /// Whether this node participates in focus navigation.
    pub focusable: bool,
    /// Optional explicit tab order index.
    pub tab_index: Option<i32>,
    /// Nodes owned by this node in the accessibility tree.
    pub aria_owns: SmallVec<[NodeId; 2]>,
    /// Node that labels this node.
    pub aria_labelled_by: Option<NodeId>,
}

impl Default for Semantics {
    fn default() -> Self {
        Self {
            role: accesskit::Role::GenericContainer,
            name: NameSource::None,
            description: None,
            actions: SmallVec::new(),
            focusable: false,
            tab_index: None,
            aria_owns: SmallVec::new(),
            aria_labelled_by: None,
        }
    }
}
