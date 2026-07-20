//! Reactive children primitive — `NodeBuilder::children_reactive`.
//!
//! Installs a single `effect_named` parented to the parent component's
//! reactive owner. On every signal change the effect enqueues a
//! deferred rebuild against a thread-local queue; `Cx::flush_reactive_children`
//! drains the queue at engine-tick time (B1.b will wire it into
//! `LayoutEngine::compute`; for B1.a tests call it explicitly).
//!
//! ## Mechanism
//!
//! 1. The user's closure is wrapped in an effect. Inside the effect we
//!    read the closure (which tracks signal dependencies) and push a
//!    `DeferredRebuild { parent, rebuild }` onto the thread-local queue.
//! 2. `flush_reactive_children` drains the queue. For each entry, it
//!    removes every prior child id (cached in a shared `RefCell`),
//!    calls the rebuild thunk to produce the fresh `Box<dyn ErasedComponent>`
//!    list, adds them as children, and updates the cache.
//! 3. Disposal cascades through the parent's component owner — when
//!    `Cx::remove(parent)` runs, the effect's owner dies with it and
//!    no further rebuilds fire.

use std::cell::RefCell;
use std::rc::Rc;

use smallvec::SmallVec;

use crate::component::{ErasedComponent, IntoNode};
use crate::{Cx, NodeId};

/// One pending children-list rebuild for a parent.
///
/// `rebuild` is invoked with `&mut Cx` during
/// `Cx::flush_reactive_children`. It returns the fresh boxed child list.
pub(crate) struct DeferredRebuild {
    pub(crate) parent: NodeId,
    pub(crate) prior: Rc<RefCell<SmallVec<[NodeId; 4]>>>,
    pub(crate) rebuild: Box<dyn FnMut() -> Vec<Box<dyn ErasedComponent>>>,
}

thread_local! {
    /// Pending rebuilds queued from inside reactive effects. Drained by
    /// `Cx::flush_reactive_children`. Single-threaded because the entire
    /// reactive runtime is main-thread only (see DR-005).
    static PENDING: RefCell<Vec<DeferredRebuild>> = const { RefCell::new(Vec::new()) };
}

/// Push a rebuild onto the thread-local queue.
fn enqueue(rebuild: DeferredRebuild) {
    PENDING.with(|cell| cell.borrow_mut().push(rebuild));
}

/// Drain the queue into the caller's vector. Used by
/// `Cx::flush_reactive_children` so the kernel never observes the
/// thread-local directly outside this module.
pub(crate) fn drain_pending() -> Vec<DeferredRebuild> {
    PENDING.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}

/// Install the reactive-children effect on `parent`.
///
/// Called from `NodeBuilder::build` after the parent node exists and
/// while we are still inside the parent's component owner scope. The
/// effect therefore registers against the per-component owner — when
/// `Cx::remove(parent)` disposes that owner, the effect dies with it.
pub(crate) fn install<F, I, N>(parent: NodeId, mut f: F)
where
    F: FnMut() -> I + 'static,
    I: IntoIterator<Item = N>,
    N: IntoNode + 'static,
{
    let prior = Rc::new(RefCell::new(SmallVec::<[NodeId; 4]>::new()));
    let prior_for_effect = Rc::clone(&prior);

    sigil_reactive::effect_named("sigil-kernel::children_reactive", move || {
        // Re-run the user closure so the effect tracks its signal deps,
        // then box the result for deferred rebuild. We can't touch `&mut Cx`
        // from here — the rebuild runs at flush time.
        let builders: Vec<Box<dyn ErasedComponent>> = f()
            .into_iter()
            .map(|child| Box::new(child) as Box<dyn ErasedComponent>)
            .collect();

        // The rebuild thunk takes ownership of `builders` once via Option;
        // FnMut allows the effect to fire multiple times across rebuilds.
        let mut slot = Some(builders);
        let rebuild: Box<dyn FnMut() -> Vec<Box<dyn ErasedComponent>>> =
            Box::new(move || slot.take().unwrap_or_default());

        enqueue(DeferredRebuild {
            parent,
            prior: Rc::clone(&prior_for_effect),
            rebuild,
        });
    });
}

impl Cx {
    /// Drain any pending reactive-children rebuilds.
    ///
    /// For each queued rebuild: remove every cached prior child of the
    /// parent, call the rebuild thunk for the fresh `Box<dyn ErasedComponent>`
    /// list, add each as a child of `parent`, and update the prior-id
    /// cache. Idempotent when the queue is empty.
    ///
    /// B1.b will call this from `LayoutEngine::compute` before the
    /// cache-key check; B1.a tests call it explicitly.
    // TODO(B1-followup): when a rebuild adds >50% of live nodes,
    // consider flipping `dirty_all` via `mark_mutated` rather than
    // paying per-child `mark_dirty` costs (see design §8 Risk 2).
    pub fn flush_reactive_children(&mut self) {
        loop {
            let pending = drain_pending();
            if pending.is_empty() {
                return;
            }
            for rebuild in pending {
                self.apply_rebuild(rebuild);
            }
            // Loop: a rebuild's `IntoNode::build` may itself install
            // more `children_reactive` effects, whose initial runs
            // enqueue further work. Drain to fixpoint.
        }
    }

    fn apply_rebuild(&mut self, mut rebuild: DeferredRebuild) {
        // Parent gone (removed earlier this tick) — skip silently;
        // the effect's owner is already disposed so this is the last
        // rebuild we will ever see for this id.
        if self.nodes.get(rebuild.parent.key()).is_none() {
            return;
        }

        let prior_ids: SmallVec<[NodeId; 4]> = std::mem::take(&mut *rebuild.prior.borrow_mut());
        for id in prior_ids {
            self.remove(id);
        }

        let builders = (rebuild.rebuild)();
        let mut new_ids = SmallVec::<[NodeId; 4]>::with_capacity(builders.len());
        for child in builders {
            new_ids.push(self.add_boxed_child(rebuild.parent, child));
        }
        *rebuild.prior.borrow_mut() = new_ids;
    }
}
