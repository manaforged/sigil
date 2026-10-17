use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

use crate::kernel::{Cx, NodeId};
use crate::kernel::{IntoNode, NodeBuilder, container, fragment, node};
use crate::reactive::{Reactive, Signal, derived};
use crate::style::NodeStyleExt;
use crate::tasks::Status;

#[must_use]
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

#[must_use]
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
    fragment().children_keyed(move || {
        items.with(|all| {
            all.iter()
                .map(|item| (key(item), render(item)))
                .collect::<Vec<_>>()
        })
    })
}

type View = Rc<dyn Fn() -> Box<dyn IntoNode>>;

fn erase<N: IntoNode + 'static>(view: impl Fn() -> N + 'static) -> View {
    Rc::new(move || node(view()))
}

#[must_use]
pub struct SwitchBuilder<K: 'static> {
    key: Box<dyn Reactive<K>>,
    cases: Vec<(K, View)>,
    otherwise: Option<View>,
    keep_alive: bool,
}

pub fn switch<K: Clone + PartialEq + 'static>(key: impl Reactive<K>) -> SwitchBuilder<K> {
    SwitchBuilder {
        key: Box::new(key),
        cases: Vec::new(),
        otherwise: None,
        keep_alive: false,
    }
}

impl<K: Clone + PartialEq + 'static> SwitchBuilder<K> {
    pub fn case<N: IntoNode + 'static>(mut self, key: K, view: impl Fn() -> N + 'static) -> Self {
        self.cases.push((key, erase(view)));
        self
    }

    pub fn otherwise<N: IntoNode + 'static>(mut self, view: impl Fn() -> N + 'static) -> Self {
        self.otherwise = Some(erase(view));
        self
    }

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
        let (keys, mut views): (Vec<K>, Vec<View>) = cases.into_iter().unzip();
        let fallback = otherwise.is_some().then_some(keys.len());
        views.extend(otherwise);
        let slot = derived(move || {
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

fn mount_kept(views: Vec<View>, slot: Signal<Option<usize>>) -> NodeBuilder {
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
                let page = container()
                    .child(move || view())
                    .visible_when(move || slot.get() == Some(i));
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

type ValueView<V> = Box<dyn Fn(&V) -> Box<dyn IntoNode>>;

#[must_use]
pub struct StatusView<T: 'static, E: 'static> {
    status: Box<dyn Reactive<Status<T, E>>>,
    idle: Option<View>,
    pending: Option<View>,
    ok: Option<ValueView<T>>,
    error: Option<ValueView<E>>,
}

pub fn match_status<T: 'static, E: 'static>(
    status: impl Reactive<Status<T, E>>,
) -> StatusView<T, E> {
    StatusView {
        status: Box::new(status),
        idle: None,
        pending: None,
        ok: None,
        error: None,
    }
}

impl<T: 'static, E: From<String> + 'static> StatusView<T, E> {
    pub fn idle<N: IntoNode + 'static>(mut self, view: impl Fn() -> N + 'static) -> Self {
        self.idle = Some(erase(view));
        self
    }

    pub fn pending<N: IntoNode + 'static>(mut self, view: impl Fn() -> N + 'static) -> Self {
        self.pending = Some(erase(view));
        self
    }

    pub fn ok<N: IntoNode + 'static>(mut self, view: impl Fn(&T) -> N + 'static) -> Self {
        self.ok = Some(Box::new(move |v| node(view(v))));
        self
    }

    pub fn error<N: IntoNode + 'static>(mut self, view: impl Fn(&E) -> N + 'static) -> Self {
        self.error = Some(Box::new(move |e| node(view(e))));
        self
    }

    fn render(&self, status: Status<T, E>) -> Option<Box<dyn IntoNode>> {
        match status {
            Status::Idle => self.idle.as_ref().map(|v| v()),
            Status::Pending => self.pending.as_ref().map(|v| v()),
            Status::Ok(value) => self.ok.as_ref().map(|v| v(&value)),
            Status::Err(err) => self.error.as_ref().map(|v| v(&err)),
            Status::Panicked(msg) => self
                .error
                .as_ref()
                .map(|v| v(&E::from(format!("task panicked: {msg}")))),
        }
    }
}

impl<T: 'static, E: From<String> + 'static> IntoNode for StatusView<T, E> {
    fn build(self, cx: &mut Cx) -> NodeId {
        let generation = Cell::new(0_u64);
        fragment()
            .children_keyed(move || {
                generation.set(generation.get() + 1);
                let child = self.render(self.status.read());
                child.map(|c| (generation.get(), c))
            })
            .build(cx)
    }
}
