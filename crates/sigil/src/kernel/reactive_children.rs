use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

use ahash::{AHashMap, AHashSet};
use smallvec::SmallVec;

use crate::kernel::builder::{Fate, Unpatched};
use crate::kernel::component::{ErasedComponent, IntoNode};
use crate::kernel::{Cx, NodeId};
use crate::reactive::{Owner, OwnerId, is_live, on_cleanup};

type RebuildFn = Box<dyn FnMut(&mut Cx)>;
pub(crate) type PriorNodes = SmallVec<[NodeId; 4]>;
type KeyedIncoming<K> = Vec<(K, Option<Box<dyn ErasedComponent>>)>;
type Fates = SmallVec<[Fate; 4]>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Survivor {
    Patch,
    Keep,
}

pub(crate) struct DeferredRebuild {
    pub(crate) parent: NodeId,
    pub(crate) apply: RebuildFn,
}

thread_local! {
    static PENDING: RefCell<Vec<DeferredRebuild>> = const { RefCell::new(Vec::new()) };
    static SEEDS: RefCell<Vec<(NodeId, PriorNodes)>> = const { RefCell::new(Vec::new()) };
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

pub(crate) fn seed(parent: NodeId, ids: PriorNodes) {
    if ids.is_empty() {
        return;
    }
    SEEDS.with(|cell| cell.borrow_mut().push((parent, ids)));
}

pub(crate) fn take_seed(parent: NodeId) -> PriorNodes {
    SEEDS.with(|cell| {
        let mut seeds = cell.borrow_mut();
        seeds
            .iter()
            .position(|(owner, _)| *owner == parent)
            .map(|index| seeds.swap_remove(index).1)
            .unwrap_or_default()
    })
}

fn generation_owner() -> Owner {
    Owner::new(
        "sigil-kernel::children-generation",
        crate::reactive::current_effect_owner().map(OwnerId),
    )
}

struct GenerationGuard(Option<Owner>);

impl GenerationGuard {
    fn superseded(&self) -> bool {
        self.0.is_some_and(|owner| !is_live(owner.id()))
    }
}

impl Drop for GenerationGuard {
    fn drop(&mut self) {
        if let Some(owner) = self.0.take() {
            dispose_if_live(owner);
        }
    }
}

fn dispose_if_live(owner: Owner) {
    if is_live(owner.id()) {
        owner.dispose();
    }
}

pub(crate) fn install<F, I, N>(parent: NodeId, mut f: F)
where
    F: FnMut() -> I + 'static,
    I: IntoIterator<Item = N>,
    N: IntoNode + 'static,
{
    let prior: Rc<RefCell<PriorNodes>> = Rc::new(RefCell::new(take_seed(parent)));
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

pub(crate) fn install_keyed<F, I, K, N>(parent: NodeId, mut f: F, survivor: Survivor)
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
        enqueue_keyed(
            parent,
            Rc::clone(&prior),
            incoming,
            survivor,
            Some(generation),
        );
    });
}

fn stale(cx: &Cx, parent: NodeId, generation: &GenerationGuard) -> bool {
    cx.nodes.get(parent.key()).is_none() || generation.superseded()
}

fn apply_positional(
    cx: &mut Cx,
    parent: NodeId,
    prior: &RefCell<PriorNodes>,
    builders: Vec<Box<dyn ErasedComponent>>,
    generation: &mut GenerationGuard,
) {
    let prior_ids = std::mem::take(&mut *prior.borrow_mut());
    if stale(cx, parent, generation) {
        return;
    }
    let old: PriorNodes = prior_ids
        .into_iter()
        .filter(|id| cx.nodes.contains_key(id.key()))
        .collect();
    for id in old.iter().skip(builders.len()) {
        cx.remove_counted(*id);
    }
    let mut fates = Fates::with_capacity(builders.len());
    for (index, builder) in builders.into_iter().enumerate() {
        let existing = old.get(index).copied();
        fates.extend(cx.reconcile_child(parent, existing, Some(builder), Unpatched::Remount));
    }
    *prior.borrow_mut() = settle(cx, parent, &fates, generation);
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
    enqueue_keyed(parent, prior, incoming, Survivor::Patch, None);
}

fn enqueue_keyed<K: Clone + Eq + Hash + 'static>(
    parent: NodeId,
    prior: Rc<RefCell<Vec<(K, NodeId)>>>,
    incoming: KeyedIncoming<K>,
    survivor: Survivor,
    generation: Option<Owner>,
) {
    let mut slot = Some(incoming);
    let mut generation = GenerationGuard(generation);
    enqueue(DeferredRebuild {
        parent,
        apply: Box::new(move |cx| {
            apply_keyed(
                cx,
                parent,
                &prior,
                slot.take().unwrap_or_default(),
                survivor,
                &mut generation,
            );
        }),
    });
}

#[expect(
    clippy::too_many_arguments,
    reason = "one keyed apply needs the list, its prior ids, the incoming items, the survivor rule, and the generation guard"
)]
fn apply_keyed<K: Clone + Eq + Hash>(
    cx: &mut Cx,
    parent: NodeId,
    prior: &RefCell<Vec<(K, NodeId)>>,
    incoming: KeyedIncoming<K>,
    survivor: Survivor,
    generation: &mut GenerationGuard,
) {
    if stale(cx, parent, generation) {
        return;
    }
    let old = std::mem::take(&mut *prior.borrow_mut());
    let mut live: AHashMap<K, NodeId> = old.into_iter().collect();
    let (mut next, fates) = mount_keyed(cx, parent, incoming, &mut live, survivor);
    for id in live.into_values() {
        cx.remove_counted(id);
    }
    settle(cx, parent, &fates, generation);
    next.retain(|(_, id)| cx.nodes.get(id.key()).is_some());
    *prior.borrow_mut() = next;
}

fn settle(
    cx: &mut Cx,
    parent: NodeId,
    fates: &[Fate],
    generation: &mut GenerationGuard,
) -> PriorNodes {
    let order: PriorNodes = fates.iter().map(|fate| fate.id()).collect();
    let keep: AHashSet<NodeId> = order.iter().copied().collect();
    let prefix = cx.others(parent, &keep);
    cx.replace_children(parent, prefix.iter().chain(&order).copied().collect());
    let fresh: PriorNodes = fates
        .iter()
        .filter(|fate| fate.is_fresh())
        .map(|fate| fate.id())
        .collect();
    pin_generation(cx, generation.0.take(), &fresh);
    order
}

fn pin_generation(cx: &Cx, generation: Option<Owner>, fresh: &[NodeId]) {
    let Some(generation) = generation else {
        return;
    };
    if fresh.is_empty() {
        dispose_if_live(generation);
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
                } else {
                    dispose_if_live(generation);
                }
            });
        });
    }
}

fn mount_keyed<K: Clone + Eq + Hash>(
    cx: &mut Cx,
    parent: NodeId,
    incoming: KeyedIncoming<K>,
    live: &mut AHashMap<K, NodeId>,
    survivor: Survivor,
) -> (Vec<(K, NodeId)>, Fates) {
    let mut seen = AHashSet::new();
    let mut next = Vec::with_capacity(incoming.len());
    let mut fates = Fates::new();
    for (key, builder) in incoming {
        if !seen.insert(key.clone()) {
            continue;
        }
        let existing = live.remove(&key);
        let builder = survivor_builder(cx, survivor, existing, builder);
        let Some(fate) = cx.reconcile_child(parent, existing, builder, Unpatched::Keep) else {
            continue;
        };
        fates.push(fate);
        next.push((key, fate.id()));
    }
    (next, fates)
}

fn survivor_builder(
    cx: &Cx,
    survivor: Survivor,
    existing: Option<NodeId>,
    builder: Option<Box<dyn ErasedComponent>>,
) -> Option<Box<dyn ErasedComponent>> {
    let alive = existing.is_some_and(|id| cx.nodes.contains_key(id.key()));
    match survivor {
        Survivor::Keep if alive => None,
        Survivor::Keep | Survivor::Patch => builder,
    }
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
