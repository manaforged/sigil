use std::cell::RefCell;
use std::rc::Rc;

use crate::reactive::owner::on_cleanup;
use crate::reactive::runtime::with_rt;

type SubEntry<T> = (u64, Rc<dyn Fn(T)>);

struct Inner<T> {
    next: u64,
    subs: Vec<SubEntry<T>>,
}

#[derive(Clone)]
pub struct EventChannel<T: 'static> {
    inner: Rc<RefCell<Inner<T>>>,
}

impl<T: Clone + 'static> EventChannel<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner {
                next: 0,
                subs: Vec::new(),
            })),
        }
    }

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
        Subscription::new(move || remove_sub(&inner, id))
    }
}

#[must_use = "subscriptions unsubscribe on drop; bind the handle to keep it alive"]
pub struct Subscription {
    cleanup: Option<Box<dyn FnOnce()>>,
    _not_send: core::marker::PhantomData<*const ()>,
}

impl Subscription {
    fn new(cleanup: impl FnOnce() + 'static) -> Self {
        Self {
            cleanup: Some(Box::new(cleanup)),
            _not_send: core::marker::PhantomData,
        }
    }
}

impl core::fmt::Debug for Subscription {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Subscription(<cleanup>)")
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
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
