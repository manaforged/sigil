//! Accessibility semantics carried by every node.

use sigil_hal::Shortcut;
use sigil_reactive::Signal;
use smallvec::SmallVec;

use crate::NodeId;

/// Source for a node's accessible name or description.
#[derive(Clone, Debug)]
pub enum AccessibleName {
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
pub struct A11yProps {
    /// AccessKit role for this node.
    pub role: accesskit::Role,
    /// Accessible name source.
    pub name: AccessibleName,
    /// Optional accessible description source.
    pub description: Option<AccessibleName>,
    /// Supported platform actions.
    pub actions: SmallVec<[accesskit::Action; 4]>,
    /// Whether this node participates in focus navigation.
    pub focusable: bool,
    /// Optional explicit tab order index.
    pub tab_index: Option<i32>,
    /// Optional keyboard shortcut advertised by the node.
    pub keyboard_shortcut: Option<Shortcut>,
    /// Whether this node participates in pointer hit-testing.
    pub hit_test_enabled: bool,
    /// Reactive checked state for checkbox-like controls.
    pub checked: Option<Signal<bool>>,
    /// Reactive numeric value for range-like controls.
    pub value_now: Option<Signal<f32>>,
    /// Minimum numeric value for range-like controls.
    pub value_min: Option<f32>,
    /// Maximum numeric value for range-like controls.
    pub value_max: Option<f32>,
    /// Reactive text value for text-input-like controls.
    pub text_value: Option<Signal<String>>,
    /// Placeholder text for text-input-like controls.
    pub placeholder: Option<String>,
    /// Nodes owned by this node in the accessibility tree.
    pub aria_owns: SmallVec<[NodeId; 2]>,
    /// Node that labels this node.
    pub aria_labelled_by: Option<NodeId>,
}

impl Default for A11yProps {
    fn default() -> Self {
        Self {
            role: accesskit::Role::GenericContainer,
            name: AccessibleName::None,
            description: None,
            actions: SmallVec::new(),
            focusable: false,
            tab_index: None,
            keyboard_shortcut: None,
            hit_test_enabled: true,
            checked: None,
            value_now: None,
            value_min: None,
            value_max: None,
            text_value: None,
            placeholder: None,
            aria_owns: SmallVec::new(),
            aria_labelled_by: None,
        }
    }
}
