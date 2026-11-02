use crate::events::Shortcut;
use crate::reactive::Signal;
use smallvec::SmallVec;

use crate::kernel::NodeId;

/// Kind of focus scope on a node: a modal traps focus and a popup closes on focus loss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusScopeKind {
    /// The modal focus scope kind.
    Modal,
    /// The popup focus scope kind.
    Popup,
}

/// None, static, or signal name.
#[derive(Clone, Debug)]
pub enum AccessibleName {
    /// The none accessible name.
    None,
    /// The static accessible name.
    Static(String),
    /// The derived accessible name.
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

/// Caret and selection anchor of an editable text node, exported to accessibility.
#[derive(Clone, Copy, Debug)]
pub struct TextCursor {
    /// The caret.
    pub caret: Signal<usize>,
    /// The anchor.
    pub anchor: Signal<Option<usize>>,
}

/// Accessibility properties of a node.
#[derive(Clone, Debug)]
pub struct A11yProps {
    /// The role.
    pub role: super::Role,
    /// The name.
    pub name: AccessibleName,
    /// The description, if any.
    pub description: Option<AccessibleName>,
    /// The actions.
    pub actions: SmallVec<[super::Action; 4]>,
    /// Whether focusable.
    pub focusable: bool,
    /// The disabled, if any.
    pub disabled: Option<Signal<bool>>,
    /// Whether traps tab.
    pub traps_tab: bool,
    /// The tab index, if any.
    pub tab_index: Option<i32>,
    /// The tab stop, if any.
    pub tab_stop: Option<Signal<bool>>,
    /// The keyboard shortcut, if any.
    pub keyboard_shortcut: Option<Shortcut>,
    /// Whether hittable.
    pub hittable: bool,
    /// The checked, if any.
    pub checked: Option<Signal<bool>>,
    /// The selected, if any.
    pub selected: Option<Signal<bool>>,
    /// The value now, if any.
    pub value_now: Option<Signal<f32>>,
    /// The value min, if any.
    pub value_min: Option<f32>,
    /// The value max, if any.
    pub value_max: Option<f32>,
    /// The step, if any.
    pub step: Option<f32>,
    /// The text value, if any.
    pub text_value: Option<Signal<String>>,
    /// The text cursor, if any.
    pub text_cursor: Option<TextCursor>,
    /// The placeholder, if any.
    pub placeholder: Option<String>,
    /// The owns.
    pub owns: SmallVec<[NodeId; 2]>,
    /// The labelled by, if any.
    pub labelled_by: Option<NodeId>,
    /// The described by, if any.
    pub described_by: Option<NodeId>,
    /// The invalid, if any.
    pub invalid: Option<Signal<bool>>,
    /// The required, if any.
    pub required: Option<Signal<bool>>,
    /// The live, if any.
    pub live: Option<super::Live>,
    /// The expanded, if any.
    pub expanded: Option<Signal<bool>>,
    /// The level, if any.
    pub level: Option<usize>,
    /// The sort direction, if any.
    pub sort_direction: Option<Signal<Option<super::SortDirection>>>,
    /// The row count, if any.
    pub row_count: Option<Signal<usize>>,
    /// The column count, if any.
    pub column_count: Option<usize>,
    /// The row index, if any.
    pub row_index: Option<usize>,
    /// The column index, if any.
    pub column_index: Option<usize>,
    /// The position in set, if any.
    pub position_in_set: Option<usize>,
    /// The size of set, if any.
    pub size_of_set: Option<usize>,
    /// The focus scope, if any.
    pub focus_scope: Option<FocusScopeKind>,
}

impl Default for A11yProps {
    fn default() -> Self {
        Self {
            role: super::Role::GenericContainer,
            name: AccessibleName::None,
            description: None,
            actions: SmallVec::new(),
            focusable: false,
            disabled: None,
            traps_tab: false,
            tab_index: None,
            tab_stop: None,
            keyboard_shortcut: None,
            hittable: true,
            checked: None,
            selected: None,
            value_now: None,
            value_min: None,
            value_max: None,
            step: None,
            text_value: None,
            text_cursor: None,
            placeholder: None,
            owns: SmallVec::new(),
            labelled_by: None,
            described_by: None,
            invalid: None,
            required: None,
            live: None,
            expanded: None,
            level: None,
            sort_direction: None,
            row_count: None,
            column_count: None,
            row_index: None,
            column_index: None,
            position_in_set: None,
            size_of_set: None,
            focus_scope: None,
        }
    }
}

impl A11yProps {
    /// Returns whether this a11y props is disabled.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled.is_some_and(Signal::peek)
    }

    /// Returns whether this a11y props is focusable.
    #[must_use]
    pub fn is_focusable(&self) -> bool {
        self.focusable && !self.is_disabled()
    }
}
