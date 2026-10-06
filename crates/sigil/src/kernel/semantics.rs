use crate::events::Shortcut;
use crate::reactive::Signal;
use smallvec::SmallVec;

use crate::kernel::NodeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusScopeKind {
    Modal,
    Popup,
    Inert,
}

#[derive(Clone, Debug)]
pub enum AccessibleName {
    None,
    Static(String),
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

#[derive(Clone, Copy, Debug)]
pub struct TextCursor {
    pub caret: Signal<usize>,
    pub anchor: Signal<Option<usize>>,
}

#[derive(Clone, Debug)]
pub struct A11yProps {
    pub role: accesskit::Role,
    pub name: AccessibleName,
    pub description: Option<AccessibleName>,
    pub actions: SmallVec<[accesskit::Action; 4]>,
    pub focusable: bool,
    pub disabled: Option<Signal<bool>>,
    pub traps_tab: bool,
    pub tab_index: Option<i32>,
    pub tab_stop: Option<Signal<bool>>,
    pub keyboard_shortcut: Option<Shortcut>,
    pub hittable: bool,
    pub checked: Option<Signal<bool>>,
    pub selected: Option<Signal<bool>>,
    pub value_now: Option<Signal<f32>>,
    pub value_min: Option<f32>,
    pub value_max: Option<f32>,
    pub step: Option<f32>,
    pub text_value: Option<Signal<String>>,
    pub text_cursor: Option<TextCursor>,
    pub placeholder: Option<String>,
    pub owns: SmallVec<[NodeId; 2]>,
    pub labelled_by: Option<NodeId>,
    pub described_by: Option<NodeId>,
    pub invalid: Option<Signal<bool>>,
    pub required: Option<Signal<bool>>,
    pub live: Option<accesskit::Live>,
    pub expanded: Option<Signal<bool>>,
    pub level: Option<usize>,
    pub sort_direction: Option<Signal<Option<accesskit::SortDirection>>>,
    pub row_count: Option<Signal<usize>>,
    pub column_count: Option<usize>,
    pub row_index: Option<usize>,
    pub column_index: Option<usize>,
    pub position_in_set: Option<usize>,
    pub size_of_set: Option<usize>,
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
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled.is_some_and(Signal::peek)
    }

    #[must_use]
    pub fn is_focusable(&self) -> bool {
        self.focusable && !self.is_disabled()
    }
}
