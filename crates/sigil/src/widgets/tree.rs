mod flatten;
mod keys;
mod row;

use crate::widgets::Widget;
use std::hash::Hash;
use std::rc::Rc;

use crate::kernel::{NodeBuilder, Role, TextContent};
use crate::reactive::{Reactive, Signal, signal};
use crate::style::{Tone, space};

use super::icon::{IconSource, IntoIconSource};
use super::layout_ext::LayoutOverrides;
use super::patterns::RowTone;
use super::shared::accessible_name;
use super::virtual_list::{SelectionMode, virtual_list};

pub(crate) use flatten::TreeRow;

const ROW_ESTIMATE: f32 = space(6);

/// One node of a `tree`.
#[derive(Clone, Debug)]
pub struct TreeNode<K: Clone + Eq + Hash + 'static> {
    /// The id.
    pub id: K,
    /// The label.
    pub label: TextContent,
    /// The leading, if any.
    pub leading: Option<IconSource>,
    /// The status, if any.
    pub status: Option<(TextContent, Tone)>,
    /// The detail, if any.
    pub detail: Option<TextContent>,
    /// The tone.
    pub tone: RowTone,
    /// The children.
    pub children: Vec<Self>,
}

impl<K: Clone + Eq + Hash + 'static> TreeNode<K> {
    /// Creates a tree node.
    #[must_use]
    pub fn new(id: K, label: impl Into<TextContent>) -> Self {
        Self {
            id,
            label: label.into(),
            leading: None,
            status: None,
            detail: None,
            tone: RowTone::Normal,
            children: Vec::new(),
        }
    }

    /// Sets the leading.
    #[must_use]
    pub fn leading(mut self, glyph: impl IntoIconSource) -> Self {
        self.leading = Some(glyph.into_icon_source());
        self
    }

    /// Sets the status.
    #[must_use]
    pub fn status(mut self, status: impl Into<TextContent>, tone: Tone) -> Self {
        self.status = Some((status.into(), tone));
        self
    }

    /// Sets the detail.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<TextContent>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Sets the tone.
    #[must_use]
    pub const fn tone(mut self, tone: RowTone) -> Self {
        self.tone = tone;
        self
    }

    /// Sets the children.
    #[must_use]
    pub fn children(mut self, children: impl IntoIterator<Item = Self>) -> Self {
        self.children = children.into_iter().collect();
        self
    }

    /// Adds one child.
    #[must_use]
    pub fn child(mut self, child: Self) -> Self {
        self.children.push(child);
        self
    }
}

type ActivateKey<K> = Rc<dyn Fn(&K)>;
type NodesFn<K> = Box<dyn Fn() -> Vec<TreeNode<K>>>;

/// Builder of a `tree`.
pub struct TreeBuilder<K: Clone + Eq + Hash + 'static> {
    nodes: NodesFn<K>,
    expanded: Option<Signal<Vec<K>>>,
    selected: Option<Signal<Option<K>>>,
    activate: Option<ActivateKey<K>>,
    label: Option<TextContent>,
    indent: Option<f32>,
    layout_overrides: LayoutOverrides,
}

impl<K: Clone + Eq + Hash + 'static> core::fmt::Debug for TreeBuilder<K> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TreeBuilder")
            .field("expanded", &self.expanded)
            .field("selected", &self.selected)
            .field("label", &self.label)
            .field("indent", &self.indent)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A tree view for explorers, outlines, and settings.
#[must_use]
pub fn tree<K: Clone + Eq + Hash + 'static>(
    nodes: impl Reactive<Vec<TreeNode<K>>>,
) -> TreeBuilder<K> {
    TreeBuilder {
        nodes: Box::new(move || nodes.read()),
        expanded: None,
        selected: None,
        activate: None,
        label: None,
        indent: None,
        layout_overrides: LayoutOverrides::new(),
    }
}

impl<K: Clone + Eq + Hash + 'static> TreeBuilder<K> {
    /// Sets the expanded.
    #[must_use]
    pub const fn expanded(mut self, expanded: Signal<Vec<K>>) -> Self {
        self.expanded = Some(expanded);
        self
    }

    /// Sets the selected.
    #[must_use]
    pub const fn selected(mut self, selected: Signal<Option<K>>) -> Self {
        self.selected = Some(selected);
        self
    }

    /// Sets the activate.
    #[must_use]
    pub fn activate(mut self, activate: impl Fn(&K) + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    /// Sets the indent.
    #[must_use]
    pub const fn indent(mut self, cols: f32) -> Self {
        self.indent = Some(cols);
        self
    }

    /// Sets the label.
    #[must_use]
    pub fn label(mut self, label: impl Into<TextContent>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn root(self) -> NodeBuilder {
        let state = TreeState {
            rows: signal(Vec::new()),
            expanded: self.expanded.unwrap_or_else(|| signal(Vec::new())),
            selected: self.selected.unwrap_or_else(|| signal(None)),
            cursor: signal(0),
            activate: self.activate,
            indent: self.indent,
        };
        flatten::bind(self.nodes, &state);
        keys::sync_cursor(&state);
        let render_state = state.clone();
        let label = self.label;
        let list = virtual_list(state.rows, move |index, item: &TreeRow<K>| {
            row::render(&render_state, index, item)
        });
        list.selection(SelectionMode::None)
            .cursor(state.cursor)
            .measured()
            .estimate(ROW_ESTIMATE)
            .viewport_fill()
            .map_node(move |node| {
                let node = keys::attach(node.role(Role::Tree), &state);
                match label {
                    Some(label) => node.label(accessible_name(&label)),
                    None => node,
                }
            })
            .into_node_builder()
    }
}

builder_impls!([K: Clone + Eq + Hash + 'static] TreeBuilder<K>: widget, node);

#[derive(Clone)]
struct TreeState<K: Clone + Eq + Hash + 'static> {
    rows: Signal<Vec<TreeRow<K>>>,
    expanded: Signal<Vec<K>>,
    selected: Signal<Option<K>>,
    cursor: Signal<usize>,
    activate: Option<ActivateKey<K>>,
    indent: Option<f32>,
}
