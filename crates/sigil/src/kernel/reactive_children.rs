use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

use ahash::{AHashMap, AHashSet};
use smallvec::SmallVec;

use crate::kernel::component::{ErasedComponent, IntoNode};
use crate::kernel::{Cx, NodeId};
use crate::reactive::{Owner, OwnerId, is_live, on_cleanup};

type RebuildFn = Box<dyn FnMut(&mut Cx)>;
type PriorNodes = SmallVec<[NodeId; 4]>;
type KeyedIncoming<K> = Vec<(K, Option<Box<dyn ErasedComponent>>)>;
type KeyedMount<K> = (
    Vec<(K, NodeId)>,
    SmallVec<[NodeId; 4]>,
    SmallVec<[NodeId; 4]>,
);

pub(crate) struct DeferredRebuild {
    pub(crate) parent: NodeId,
    pub(crate) apply: RebuildFn,
}

thread_local! {
    static PENDING: RefCell<Vec<DeferredRebuild>> = const { RefCell::new(Vec::new()) };
}

fn enqueue(rebuild: DeferredRebuild) {
    let replaced = PENDING.with(|cell| {
        let mut pending = cell.borrow_mut();
        if let Some(queued) = pending
            .iter_mut()
            .rev()
            .find(|queued| queued.parent == rebuild.parent)
        {
            Some(std::mem::replace(queued, rebuild))
        } else {
            pending.push(rebuild);
            None
        }
    });
    drop(replaced);
}

pub(crate) fn drain_pending() -> Vec<DeferredRebuild> {
    PENDING.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}

fn generation_owner() -> Owner {
    Owner::new(
        "sigil-kernel::children-generation",
        crate::reactive::current_effect_owner().map(OwnerId),
    )
}

struct GenerationGuard(Option<Owner>);

impl GenerationGuard {
    const fn take(&mut self) -> Option<Owner> {
        self.0.take()
    }
}

impl Drop for GenerationGuard {
    fn drop(&mut self) {
        if let Some(owner) = self.0.take()
            && is_live(owner.id())
        {
            owner.dispose();
        }
    }
}

pub(crate) fn install<F, I, N>(parent: NodeId, mut f: F)
where
    F: FnMut() -> I + 'static,
    I: IntoIterator<Item = N>,
    N: IntoNode + 'static,
{
    let prior: Rc<RefCell<PriorNodes>> = Rc::new(RefCell::new(SmallVec::<[NodeId; 4]>::new()));
    crate::reactive::effect_named("sigil-kernel::children_reactive", move || {
        let generation = generation_owner();
        let builders: Vec<Box<dyn ErasedComponent>> = generation.with(|| {
            f().into_iter()
                .map(|child| Box::new(child) as Box<dyn ErasedComponent>)
                .collect()
        });
        let prior = Rc::clone(&prior);
        let mut slot = Some(builders);
        let mut generation = GenerationGuard(Some(generation));
        enqueue(DeferredRebuild {
            parent,
            apply: Box::new(move |cx| {
                apply_positional(
                    cx,
                    parent,
                    &prior,
                    slot.take().unwrap_or_default(),
                    &mut generation,
                );
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
    crate::reactive::effect_named("sigil-kernel::children_keyed", move || {
        let generation = generation_owner();
        let incoming: KeyedIncoming<K> = generation.with(|| {
            f().into_iter()
                .map(|(key, child)| (key, Some(Box::new(child) as Box<dyn ErasedComponent>)))
                .collect()
        });
        let prior = Rc::clone(&prior);
        let mut slot = Some(incoming);
        let mut generation = GenerationGuard(Some(generation));
        enqueue(DeferredRebuild {
            parent,
            apply: Box::new(move |cx| {
                apply_keyed(
                    cx,
                    parent,
                    &prior,
                    slot.take().unwrap_or_default(),
                    &mut generation,
                );
            }),
        });
    });
}

fn apply_positional(
    cx: &mut Cx,
    parent: NodeId,
    prior: &RefCell<PriorNodes>,
    builders: Vec<Box<dyn ErasedComponent>>,
    generation: &mut GenerationGuard,
) {
    let prior_ids = std::mem::take(&mut *prior.borrow_mut());
    if cx.nodes.get(parent.key()).is_none() {
        return;
    }
    for id in prior_ids {
        cx.remove(id);
    }
    let mut new_ids = SmallVec::<[NodeId; 4]>::with_capacity(builders.len());
    for child in builders {
        new_ids.push(cx.add_boxed_child(parent, child));
    }
    pin_generation(cx, generation.take(), &new_ids);
    *prior.borrow_mut() = new_ids;
}

pub fn schedule_keyed_rebuild<K, N>(
    parent: NodeId,
    prior: Rc<RefCell<Vec<(K, NodeId)>>>,
    incoming: Vec<(K, Option<N>)>,
) where
    K: Clone + Eq + Hash + 'static,
    N: IntoNode + 'static,
{
    let incoming: KeyedIncoming<K> = incoming
        .into_iter()
        .map(|(key, child)| {
            (
                key,
                child.map(|child| Box::new(child) as Box<dyn ErasedComponent>),
            )
        })
        .collect();
    let mut slot = Some(incoming);
    let mut generation = GenerationGuard(None);
    enqueue(DeferredRebuild {
        parent,
        apply: Box::new(move |cx| {
            apply_keyed(
                cx,
                parent,
                &prior,
                slot.take().unwrap_or_default(),
                &mut generation,
            );
        }),
    });
}

fn apply_keyed<K: Clone + Eq + Hash>(
    cx: &mut Cx,
    parent: NodeId,
    prior: &RefCell<Vec<(K, NodeId)>>,
    incoming: KeyedIncoming<K>,
    generation: &mut GenerationGuard,
) {
    if cx.nodes.get(parent.key()).is_none() {
        return;
    }
    let old = std::mem::take(&mut *prior.borrow_mut());
    let mut live: AHashMap<K, NodeId> = old.into_iter().collect();
    let (next, order, fresh) = mount_keyed(cx, parent, incoming, &mut live);
    for id in live.into_values() {
        cx.remove(id);
    }
    let keyed: AHashSet<NodeId> = order.iter().copied().collect();
    let prefix = static_prefix(cx, parent, &keyed);
    cx.replace_children(parent, keyed_children(&prefix, &order));
    let mut next = next;
    next.retain(|(_, id)| cx.nodes.get(id.key()).is_some());
    *prior.borrow_mut() = next;
    pin_generation(cx, generation.take(), &fresh);
}

fn pin_generation(cx: &Cx, generation: Option<Owner>, fresh: &[NodeId]) {
    let Some(generation) = generation else {
        return;
    };
    if fresh.is_empty() {
        if is_live(generation.id()) {
            generation.dispose();
        }
        return;
    }
    let mut owners = SmallVec::<[Owner; 4]>::new();
    for id in fresh {
        let Some(owner) = cx.owner_for_node.get(id).copied() else {
            continue;
        };
        if owners.iter().any(|seen: &Owner| seen.id() == owner.id()) {
            continue;
        }
        owners.push(owner);
    }
    if owners.is_empty() {
        return;
    }
    let remaining = Rc::new(Cell::new(owners.len()));
    for owner in owners {
        let remaining = Rc::clone(&remaining);
        owner.with(|| {
            on_cleanup(move || {
                if remaining.get() > 1 {
                    remaining.set(remaining.get() - 1);
                } else if is_live(generation.id()) {
                    generation.dispose();
                }
            });
        });
    }
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
    incoming: KeyedIncoming<K>,
    live: &mut AHashMap<K, NodeId>,
) -> KeyedMount<K> {
    let mut seen = AHashSet::new();
    let mut next = Vec::with_capacity(incoming.len());
    let mut order = SmallVec::<[NodeId; 4]>::new();
    let mut fresh = SmallVec::<[NodeId; 4]>::new();
    for (key, builder) in incoming {
        if !seen.insert(key.clone()) {
            continue;
        }
        let Some((id, mounted)) = take_or_mount(cx, parent, live.remove(&key), builder) else {
            continue;
        };
        if mounted {
            fresh.push(id);
        }
        order.push(id);
        next.push((key, id));
    }
    (next, order, fresh)
}

fn take_or_mount(
    cx: &mut Cx,
    parent: NodeId,
    existing: Option<NodeId>,
    builder: Option<Box<dyn ErasedComponent>>,
) -> Option<(NodeId, bool)> {
    match existing {
        Some(id) if cx.nodes.get(id.key()).is_some() => Some((id, false)),
        _ => builder.map(|builder| (cx.add_boxed_child(parent, builder), true)),
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
