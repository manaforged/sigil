//! Accessibility semantics carried by every node.

use sigil_hal::Shortcut;
use sigil_reactive::Signal;
use smallvec::SmallVec;

use crate::NodeId;

/// Scope kind for focus-trap regions (DR-050 §6).
///
/// Hosted by the kernel so widgets can advertise modal / popup
/// containment; the focus engine reads this when routing keyboard
/// navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusScopeKind {
    /// A modal dialog — focus traps inside the subtree.
    Modal,
    /// A popup/menu — focus traps inside the subtree but Escape
    /// dismisses to the trigger.
    Popup,
    /// Inert subtree — descendants are excluded from focus
    /// navigation but remain visible.
    Inert,
}

/// Source for a node's accessible name or description.
#[derive(Clone, Debug)]
pub enum AccessibleName {
    /// No explicit source has been provided.
    None,
    /// A fixed string supplied by the component author.
    Static(String),
    /// A reactive string owned by the component's reactive owner.
    Derived(Signal<String>),
}

impl From<String> for AccessibleName {
    fn from(value: String) -> Self {
        Self::Static(value)
    }
}

impl From<&'static str> for AccessibleName {
    fn from(value: &'static str) -> Self {
        Self::Static(value.to_owned())
    }
}

impl From<Signal<String>> for AccessibleName {
    fn from(value: Signal<String>) -> Self {
        Self::Derived(value)
    }
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
    pub hittable: bool,
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
    pub owns: SmallVec<[NodeId; 2]>,
    /// Node that labels this node.
    pub labelled_by: Option<NodeId>,
    /// Live-region politeness for assistive-tech announcements.
    pub live: Option<accesskit::Live>,
    /// Reactive expanded state for disclosure-like controls.
    pub expanded: Option<Signal<bool>>,
    /// Focus-scope kind for trap regions (modal / popup / inert).
    pub focus_scope: Option<FocusScopeKind>,
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
            hittable: true,
            checked: None,
            value_now: None,
            value_min: None,
            value_max: None,
            text_value: None,
            placeholder: None,
            owns: SmallVec::new(),
            labelled_by: None,
            live: None,
            expanded: None,
            focus_scope: None,
        }
    }
}
