mod flatten;
mod keys;
mod row;

use std::hash::Hash;
use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeId, Role, TextContent};
use crate::reactive::{Reactive, Signal, signal};
use crate::style::{Tone, space};

use super::icon::{IconSource, IntoIconSource};
use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::patterns::RowTone;
use super::virtual_list::{SelectionMode, virtual_list};

pub use flatten::TreeRow;

const ROW_ESTIMATE: f32 = space(6);

pub trait TreeKey: Clone + Eq + Hash + 'static {}

impl<K: Clone + Eq + Hash + 'static> TreeKey for K {}

#[derive(Clone)]
pub struct TreeNode<K: TreeKey> {
    pub id: K,
    pub label: TextContent,
    pub leading: Option<IconSource>,
    pub status: Option<(TextContent, Tone)>,
    pub detail: Option<TextContent>,
    pub tone: RowTone,
    pub children: Vec<Self>,
}

impl<K: TreeKey> TreeNode<K> {
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

    #[must_use]
    pub fn leading(mut self, glyph: impl IntoIconSource) -> Self {
        self.leading = Some(glyph.into_icon_source());
        self
    }

    #[must_use]
    pub fn status(mut self, status: impl Into<TextContent>, tone: Tone) -> Self {
        self.status = Some((status.into(), tone));
        self
    }

    #[must_use]
    pub fn detail(mut self, detail: impl Into<TextContent>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    #[must_use]
    pub const fn tone(mut self, tone: RowTone) -> Self {
        self.tone = tone;
        self
    }

    #[must_use]
    pub fn children(mut self, children: impl IntoIterator<Item = Self>) -> Self {
        self.children = children.into_iter().collect();
        self
    }

    #[must_use]
    pub fn child(mut self, child: Self) -> Self {
        self.children.push(child);
        self
    }
}

type Activate<K> = Rc<dyn Fn(&K)>;
type NodesFn<K> = Box<dyn Fn() -> Vec<TreeNode<K>>>;

pub struct TreeBuilder<K: TreeKey> {
    nodes: NodesFn<K>,
    expanded: Option<Signal<Vec<K>>>,
    selected: Option<Signal<Option<K>>>,
    activate: Option<Activate<K>>,
    label: Option<TextContent>,
    indent: Option<f32>,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn tree<K: TreeKey>(nodes: impl Reactive<Vec<TreeNode<K>>>) -> TreeBuilder<K> {
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

impl<K: TreeKey> TreeBuilder<K> {
    #[must_use]
    pub const fn expanded(mut self, expanded: Signal<Vec<K>>) -> Self {
        self.expanded = Some(expanded);
        self
    }

    #[must_use]
    pub const fn selected(mut self, selected: Signal<Option<K>>) -> Self {
        self.selected = Some(selected);
        self
    }

    #[must_use]
    pub fn activate(mut self, activate: impl Fn(&K) + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    #[must_use]
    pub const fn indent(mut self, cols: f32) -> Self {
        self.indent = Some(cols);
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<TextContent>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn into_list(self) -> impl IntoNode + 'static {
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
        let key_state = state.clone();
        let label = self.label;
        let mut list = virtual_list(state.rows, move |index, item: &TreeRow<K>| {
            row::render(&render_state, index, item)
        });
        *list.layout_overrides_mut() = self.layout_overrides;
        list.selection(SelectionMode::None)
            .cursor(state.cursor)
            .measured()
            .estimate(ROW_ESTIMATE)
            .viewport_fill()
            .map_node(move |node| {
                let node = keys::attach(node.role(Role::Tree), &key_state);
                match label {
                    Some(label) => node.label(row::accessible_name(&label)),
                    None => node,
                }
            })
    }
}

impl<K: TreeKey> WidgetLayoutExt for TreeBuilder<K> {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl<K: TreeKey> IntoNode for TreeBuilder<K> {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_list().build(cx)
    }
}

struct TreeState<K: TreeKey> {
    rows: Signal<Vec<TreeRow<K>>>,
    expanded: Signal<Vec<K>>,
    selected: Signal<Option<K>>,
    cursor: Signal<usize>,
    activate: Option<Activate<K>>,
    indent: Option<f32>,
}

impl<K: TreeKey> Clone for TreeState<K> {
    fn clone(&self) -> Self {
        Self {
            rows: self.rows,
            expanded: self.expanded,
            selected: self.selected,
            cursor: self.cursor,
            activate: self.activate.clone(),
            indent: self.indent,
        }
    }
}
