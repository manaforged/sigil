use crate::events::Shortcut;
use crate::reactive::Signal;
use smallvec::SmallVec;
use std::rc::Rc;

use crate::kernel::NodeId;

/// Kind of focus scope on a node: a modal traps focus and a popup closes on focus loss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusScopeKind {
    /// Traps focus inside the node, and screen readers treat the node as modal.
    Modal,
    /// Closes when focus leaves the node.
    Popup,
}

/// None, static, or signal name.
#[derive(Clone, Debug)]
pub enum AccessibleName {
    /// No name.
    None,
    /// A name that does not change.
    Static(String),
    /// A name read from a signal each time the accessibility tree is built.
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
    /// The byte offset of the caret in the text value.
    pub caret: Signal<usize>,
    /// The byte offset of the other end of the selection; `None` in the signal means no selection.
    pub anchor: Signal<Option<usize>>,
}

/// Accessibility properties of a node.
#[derive(Clone, Debug)]
pub struct A11yProps {
    /// What the node is to a screen reader; the default is `GenericContainer`.
    pub role: super::Role,
    /// The name that a screen reader speaks for the node.
    pub name: AccessibleName,
    /// True when the node can take keyboard focus; the default is false.
    pub focusable: bool,
    /// True when a Tab press goes to the focused node instead of moving focus to the next node.
    pub traps_tab: bool,
    /// True when pointer hit tests and focus moves can land on the node; the default is true.
    pub hittable: bool,
    /// The focus scope that the node opens; `None` for a node that opens none.
    pub focus_scope: Option<FocusScopeKind>,
    pub(crate) detail: Rc<A11yDetail>,
}

/// The less common accessibility properties of a node, read through `A11yProps`.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct A11yDetail {
    /// The longer text that a screen reader speaks after the name.
    pub description: Option<AccessibleName>,
    /// The actions that a screen reader can ask the node to do.
    pub actions: SmallVec<[super::Action; 4]>,
    /// The signal that is true while the node is disabled; a disabled node takes no focus.
    pub disabled: Option<Signal<bool>>,
    /// The place in the tab order: lower indexes come first, and a negative index skips the node.
    pub tab_index: Option<i32>,
    /// The signal that is true while Tab can stop on the node; the focused node stays in the order.
    pub tab_stop: Option<Signal<bool>>,
    /// The shortcut that the node reports to assistive technology.
    pub keyboard_shortcut: Option<Shortcut>,
    /// The signal that is true while the node is checked.
    pub checked: Option<Signal<bool>>,
    /// The signal that is true while the node is neither checked nor unchecked; it needs `checked`.
    pub mixed: Option<Signal<bool>>,
    /// The signal that is true while the node is selected.
    pub selected: Option<Signal<bool>>,
    /// The signal that holds the numeric value; reported only with `value_min` and `value_max`.
    pub value_now: Option<Signal<f32>>,
    /// The lowest numeric value that the node reports.
    pub value_min: Option<f32>,
    /// The highest numeric value that the node reports.
    pub value_max: Option<f32>,
    /// The amount by which one step changes the numeric value.
    pub step: Option<f32>,
    /// The signal that holds the text of the node, such as the content of a text field.
    pub text_value: Option<Signal<String>>,
    /// The caret and the selection anchor of the text; reported only with `text_value`.
    pub text_cursor: Option<TextCursor>,
    /// The hint text that the node reports for an empty field.
    pub placeholder: Option<String>,
    /// The nodes that this node names as owned, in the order they were added.
    pub owns: SmallVec<[NodeId; 2]>,
    /// The node that labels this node; when set, `name` is not reported.
    pub labelled_by: Option<NodeId>,
    /// The node that describes this node.
    pub described_by: Option<NodeId>,
    /// The signal that is true while the value of the node fails validation.
    pub invalid: Option<Signal<bool>>,
    /// The signal that is true while the user must fill in the node.
    pub required: Option<Signal<bool>>,
    /// How a screen reader announces changes to the node; `None` uses the default of the role.
    pub live: Option<super::Live>,
    /// The signal that is true while the node is expanded; it adds the collapse or expand action.
    pub expanded: Option<Signal<bool>>,
    /// The heading rank or the depth in a tree, 1-based.
    pub level: Option<usize>,
    /// The signal that holds the sort direction of a column; `None` in the signal means not sorted.
    pub sort_direction: Option<Signal<Option<super::SortDirection>>>,
    /// The signal that holds the number of rows of a table or a grid.
    pub row_count: Option<Signal<usize>>,
    /// The number of columns of a table or a grid.
    pub column_count: Option<usize>,
    /// The row that the node is in, 1-based, where the header row is 1.
    pub row_index: Option<usize>,
    /// The column that the node is in, 1-based.
    pub column_index: Option<usize>,
    /// The place of the node in its set, 1-based.
    pub position_in_set: Option<usize>,
    /// The number of items in the set that the node belongs to.
    pub size_of_set: Option<usize>,
}

thread_local! {
    static EMPTY: Rc<A11yDetail> = Rc::new(A11yDetail::new());
}

impl A11yDetail {
    fn new() -> Self {
        Self {
            description: None,
            actions: SmallVec::new(),
            disabled: None,
            tab_index: None,
            tab_stop: None,
            keyboard_shortcut: None,
            checked: None,
            mixed: None,
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
        }
    }
}

impl Default for A11yProps {
    fn default() -> Self {
        Self {
            role: super::Role::GenericContainer,
            name: AccessibleName::None,
            focusable: false,
            traps_tab: false,
            hittable: true,
            focus_scope: None,
            detail: EMPTY.with(Rc::clone),
        }
    }
}

impl core::ops::Deref for A11yProps {
    type Target = A11yDetail;

    fn deref(&self) -> &A11yDetail {
        &self.detail
    }
}

impl core::ops::DerefMut for A11yProps {
    fn deref_mut(&mut self) -> &mut A11yDetail {
        Rc::make_mut(&mut self.detail)
    }
}

impl A11yProps {
    pub(crate) fn into_detail(self) -> Option<A11yDetail> {
        let untouched = EMPTY.with(|empty| Rc::ptr_eq(empty, &self.detail));
        (!untouched).then(|| Rc::unwrap_or_clone(self.detail))
    }

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
