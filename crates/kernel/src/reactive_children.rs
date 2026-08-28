use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use ahash::{AHashMap, AHashSet};
use smallvec::SmallVec;

use crate::component::{ErasedComponent, IntoNode};
use crate::{Cx, NodeId};

type RebuildFn = Box<dyn FnMut(&mut Cx)>;
type KeyedMount<K> = (Vec<(K, NodeId)>, SmallVec<[NodeId; 4]>);

pub(crate) struct DeferredRebuild {
    pub(crate) parent: NodeId,
    pub(crate) apply: RebuildFn,
}

thread_local! {
    static PENDING: RefCell<Vec<DeferredRebuild>> = const { RefCell::new(Vec::new()) };
}

fn enqueue(rebuild: DeferredRebuild) {
    PENDING.with(|cell| {
        let mut pending = cell.borrow_mut();
        if let Some(queued) = pending
            .iter_mut()
            .rev()
            .find(|queued| queued.parent == rebuild.parent)
        {
            *queued = rebuild;
        } else {
            pending.push(rebuild);
        }
    });
}

pub(crate) fn drain_pending() -> Vec<DeferredRebuild> {
    PENDING.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}

pub(crate) fn install<F, I, N>(parent: NodeId, mut f: F)
where
    F: FnMut() -> I + 'static,
    I: IntoIterator<Item = N>,
    N: IntoNode + 'static,
{
    let prior = Rc::new(RefCell::new(SmallVec::<[NodeId; 4]>::new()));
    sigil_reactive::effect_named("sigil-kernel::children_reactive", move || {
        let builders: Vec<Box<dyn ErasedComponent>> = f()
            .into_iter()
            .map(|child| Box::new(child) as Box<dyn ErasedComponent>)
            .collect();
        let prior = Rc::clone(&prior);
        let mut slot = Some(builders);
        enqueue(DeferredRebuild {
            parent,
            apply: Box::new(move |cx| {
                apply_positional(cx, parent, &prior, slot.take().unwrap_or_default());
            }),
        });
    });
}

pub(crate) fn install_keyed<F, I, K, N>(parent: NodeId, mut f: F)
where
    F: FnMut() -> I + 'static,
    I: IntoIterator<Item = (K, N)>,
    K: Clone + Eq + Hash + 'static,
    N: IntoNode + 'static,
{
    let prior = Rc::new(RefCell::new(Vec::<(K, NodeId)>::new()));
    sigil_reactive::effect_named("sigil-kernel::children_keyed", move || {
        let incoming: Vec<(K, Box<dyn ErasedComponent>)> = f()
            .into_iter()
            .map(|(key, child)| (key, Box::new(child) as Box<dyn ErasedComponent>))
            .collect();
        let prior = Rc::clone(&prior);
        let mut slot = Some(incoming);
        enqueue(DeferredRebuild {
            parent,
            apply: Box::new(move |cx| {
                apply_keyed(cx, parent, &prior, slot.take().unwrap_or_default());
            }),
        });
    });
}

fn apply_positional(
    cx: &mut Cx,
    parent: NodeId,
    prior: &RefCell<SmallVec<[NodeId; 4]>>,
    builders: Vec<Box<dyn ErasedComponent>>,
) {
    if cx.nodes.get(parent.key()).is_none() {
        return;
    }
    let prior_ids: SmallVec<[NodeId; 4]> = std::mem::take(&mut *prior.borrow_mut());
    for id in prior_ids {
        cx.remove(id);
    }
    let mut new_ids = SmallVec::<[NodeId; 4]>::with_capacity(builders.len());
    for child in builders {
        new_ids.push(cx.add_boxed_child(parent, child));
    }
    *prior.borrow_mut() = new_ids;
}

pub fn schedule_keyed_rebuild<K, N>(
    parent: NodeId,
    prior: Rc<RefCell<Vec<(K, NodeId)>>>,
    incoming: Vec<(K, N)>,
) where
    K: Clone + Eq + Hash + 'static,
    N: IntoNode + 'static,
{
    let incoming: Vec<(K, Box<dyn ErasedComponent>)> = incoming
        .into_iter()
        .map(|(key, child)| (key, Box::new(child) as Box<dyn ErasedComponent>))
        .collect();
    let mut slot = Some(incoming);
    enqueue(DeferredRebuild {
        parent,
        apply: Box::new(move |cx| {
            apply_keyed(cx, parent, &prior, slot.take().unwrap_or_default());
        }),
    });
}

fn apply_keyed<K: Clone + Eq + Hash>(
    cx: &mut Cx,
    parent: NodeId,
    prior: &RefCell<Vec<(K, NodeId)>>,
    incoming: Vec<(K, Box<dyn ErasedComponent>)>,
) {
    if cx.nodes.get(parent.key()).is_none() {
        return;
    }
    let old = std::mem::take(&mut *prior.borrow_mut());
    if old.is_empty() {
        let (next, order) = mount_first(cx, parent, incoming);
        cx.replace_children(parent, order);
        *prior.borrow_mut() = next;
        return;
    }
    let mut live: AHashMap<K, NodeId> = old.into_iter().collect();
    let kept: AHashSet<NodeId> = live.values().copied().collect();
    let prefix = static_prefix(cx, parent, &kept);
    let (next, order) = mount_keyed(cx, parent, incoming, &mut live);
    for id in live.into_values() {
        cx.remove(id);
    }
    cx.replace_children(parent, keyed_children(&prefix, &order));
    *prior.borrow_mut() = next;
}

fn mount_first<K: Clone + Eq + Hash>(
    cx: &mut Cx,
    parent: NodeId,
    incoming: Vec<(K, Box<dyn ErasedComponent>)>,
) -> KeyedMount<K> {
    let mut seen = AHashSet::new();
    let mut next = Vec::with_capacity(incoming.len());
    let mut order = SmallVec::<[NodeId; 4]>::new();
    for (key, builder) in incoming {
        if !seen.insert(key.clone()) {
            continue;
        }
        let id = cx.add_boxed_child(parent, builder);
        order.push(id);
        next.push((key, id));
    }
    (next, order)
}

fn static_prefix(cx: &Cx, parent: NodeId, keyed: &AHashSet<NodeId>) -> SmallVec<[NodeId; 4]> {
    cx.nodes
        .get(parent.key())
        .map(|node| {
            node.children
                .iter()
                .copied()
                .filter(|id| !keyed.contains(id))
                .collect()
        })
        .unwrap_or_default()
}

fn mount_keyed<K: Clone + Eq + Hash>(
    cx: &mut Cx,
    parent: NodeId,
    incoming: Vec<(K, Box<dyn ErasedComponent>)>,
    live: &mut AHashMap<K, NodeId>,
) -> KeyedMount<K> {
    let mut seen = AHashSet::new();
    let mut next = Vec::with_capacity(incoming.len());
    let mut order = SmallVec::<[NodeId; 4]>::new();
    for (key, builder) in incoming {
        if !seen.insert(key.clone()) {
            continue;
        }
        let id = take_or_mount(cx, parent, live.remove(&key), builder);
        order.push(id);
        next.push((key, id));
    }
    (next, order)
}

fn take_or_mount(
    cx: &mut Cx,
    parent: NodeId,
    existing: Option<NodeId>,
    builder: Box<dyn ErasedComponent>,
) -> NodeId {
    match existing {
        Some(id) if cx.nodes.get(id.key()).is_some() => id,
        _ => cx.add_boxed_child(parent, builder),
    }
}

fn keyed_children(prefix: &[NodeId], order: &[NodeId]) -> SmallVec<[NodeId; 4]> {
    let mut children = SmallVec::<[NodeId; 4]>::with_capacity(prefix.len() + order.len());
    children.extend_from_slice(prefix);
    children.extend_from_slice(order);
    children
}

impl Cx {
    pub fn flush_reactive_children(&mut self) {
        loop {
            let pending = drain_pending();
            if pending.is_empty() {
                return;
            }
            for mut rebuild in pending {
                (rebuild.apply)(self);
            }
        }
    }
}
