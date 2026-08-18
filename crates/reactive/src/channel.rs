//! Edge-triggered event channels.

use std::cell::RefCell;
use std::rc::Rc;

use crate::entity_observer::Subscription;
use crate::owner::on_cleanup;
use crate::runtime::with_rt;

type SubEntry<T> = (u64, Rc<dyn Fn(T)>);

struct Inner<T> {
    next: u64,
    subs: Vec<SubEntry<T>>,
}

/// An edge-triggered one-shot broadcast channel.
///
/// `emit` notifies current subscribers once and does not retain `T`.
/// Use [`EventChannel`] for fire-once edges (menu click, tray click).
/// Use [`Signal<T>`](crate::Signal) when callers need a current value.
#[derive(Clone)]
pub struct EventChannel<T: 'static> {
    inner: Rc<RefCell<Inner<T>>>,
}

impl<T: Clone + 'static> EventChannel<T> {
    /// Create an empty channel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner {
                next: 0,
                subs: Vec::new(),
            })),
        }
    }

    /// Broadcast `value` to current subscribers. The value is not stored.
    pub fn emit(&self, value: T) {
        let subs: Vec<Rc<dyn Fn(T)>> = self
            .inner
            .borrow()
            .subs
            .iter()
            .map(|(_, f)| Rc::clone(f))
            .collect();
        for f in subs {
            f(value.clone());
        }
    }

    /// Register a subscriber. Drop the [`Subscription`] to unregister.
    ///
    /// If a reactive owner is active, disposing that owner also
    /// unregisters the callback.
    #[must_use = "dropping the subscription immediately unregisters the callback"]
    pub fn subscribe(&self, f: impl Fn(T) + 'static) -> Subscription {
        let id = {
            let mut inner = self.inner.borrow_mut();
            let id = inner.next;
            inner.next = inner.next.saturating_add(1);
            inner.subs.push((id, Rc::new(f)));
            id
        };
        if with_rt(|rt| rt.current_owner.is_some()) {
            let inner = Rc::clone(&self.inner);
            on_cleanup(move || remove_sub(&inner, id));
        }
        let inner = Rc::clone(&self.inner);
        Subscription::cleanup(move || remove_sub(&inner, id))
    }
}

fn remove_sub<T: 'static>(inner: &RefCell<Inner<T>>, id: u64) {
    inner.borrow_mut().subs.retain(|(sid, _)| *sid != id);
}

impl<T: Clone + 'static> Default for EventChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}
