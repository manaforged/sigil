use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use crate::kernel::{Cx, NodeId};
use crate::kernel::{IntoNode, NodeBuilder, container, fragment, node};
use crate::reactive::{Reactive, Signal, memo};

/// Mounts `then()` while `cond` is true and `otherwise()` while it is false.
pub fn show<A, B>(
    cond: impl Reactive<bool>,
    then: impl Fn() -> A + 'static,
    otherwise: impl Fn() -> B + 'static,
) -> NodeBuilder
where
    A: IntoNode + 'static,
    B: IntoNode + 'static,
{
    fragment().keyed_children(move || {
        let branch = cond.read();
        let child = if branch {
            node(then())
        } else {
            node(otherwise())
        };
        [(branch, child)]
    })
}

/// One child per item, reconciled by key; `render` runs once for each new key.
pub fn for_each<T, K, N>(
    items: Signal<Vec<T>>,
    key: impl Fn(&T) -> K + 'static,
    render: impl Fn(&T) -> N + 'static,
) -> NodeBuilder
where
    T: 'static,
    K: Clone + Eq + Hash + 'static,
    N: IntoNode + 'static,
{
    fragment().keyed_children_unmounted(move |mounted| {
        items.with(|all| {
            all.iter()
                .map(|item| {
                    let key = key(item);
                    let row = (!mounted(&key)).then(|| render(item));
                    (key, row)
                })
                .collect::<Vec<_>>()
        })
    })
}

type ViewFn = Rc<dyn Fn() -> Box<dyn IntoNode>>;

fn erase<N: IntoNode + 'static>(view: impl Fn() -> N + 'static) -> ViewFn {
    Rc::new(move || node(view()))
}

/// Shows one view for the current value of a key; `switch` creates it.
#[must_use]
pub struct SwitchBuilder<K: 'static> {
    key: Box<dyn Reactive<K>>,
    cases: Vec<(K, ViewFn)>,
    otherwise: Option<ViewFn>,
    keep_alive: bool,
}

impl<K: 'static> core::fmt::Debug for SwitchBuilder<K> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SwitchBuilder")
            .field("keep_alive", &self.keep_alive)
            .finish_non_exhaustive()
    }
}

/// Mounts the view of the case whose key equals `key`, or `otherwise` when no case matches.
pub fn switch<K: Clone + PartialEq + 'static>(key: impl Reactive<K>) -> SwitchBuilder<K> {
    SwitchBuilder {
        key: Box::new(key),
        cases: Vec::new(),
        otherwise: None,
        keep_alive: false,
    }
}

impl<K: Clone + PartialEq + 'static> SwitchBuilder<K> {
    /// Adds the view of one key.
    pub fn case<N: IntoNode + 'static>(mut self, key: K, view: impl Fn() -> N + 'static) -> Self {
        self.cases.push((key, erase(view)));
        self
    }

    /// Sets the view of every other key.
    pub fn otherwise<N: IntoNode + 'static>(mut self, view: impl Fn() -> N + 'static) -> Self {
        self.otherwise = Some(erase(view));
        self
    }

    /// Sets whether hidden views stay mounted.
    pub const fn keep_alive(mut self, keep: bool) -> Self {
        self.keep_alive = keep;
        self
    }

    fn into_builder(self) -> NodeBuilder {
        let Self {
            key,
            cases,
            otherwise,
            keep_alive,
        } = self;
        let (keys, mut views): (Vec<K>, Vec<ViewFn>) = cases.into_iter().unzip();
        let fallback = otherwise.is_some().then_some(keys.len());
        views.extend(otherwise);
        let slot = memo(move || {
            let current = key.read();
            keys.iter().position(|k| *k == current).or(fallback)
        });
        if keep_alive {
            mount_kept(views, slot)
        } else {
            fragment().keyed_children(move || {
                slot.get()
                    .and_then(|i| views.get(i).map(|view| (i, view())))
            })
        }
    }
}

fn mount_kept(views: Vec<ViewFn>, slot: Signal<Option<usize>>) -> NodeBuilder {
    let visited = RefCell::new(Vec::<usize>::new());
    fragment().keyed_children(move || {
        let mut seen = visited.borrow_mut();
        if let Some(active) = slot.get()
            && !seen.contains(&active)
        {
            seen.push(active);
        }
        seen.iter()
            .filter_map(|&i| views.get(i).map(|view| (i, Rc::clone(view))))
            .map(|(i, view)| {
                let page = crate::style::builder_ext::visible(
                    container().child(move || view()),
                    move || slot.get() == Some(i),
                );
                (i, page)
            })
            .collect::<Vec<_>>()
    })
}

impl<K: Clone + PartialEq + 'static> IntoNode for SwitchBuilder<K> {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_builder().build(cx)
    }
}
