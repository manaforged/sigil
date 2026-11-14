use core::any::Any;
use core::panic::Location;
use std::rc::Rc;

use crate::reactive::diagnostic::no_scope;
use crate::reactive::runtime::{OwnerKey, RuntimeState, with_rt};

pub(crate) type Provided = Vec<Rc<dyn Any>>;

/// Gives `value` to the current owner and every owner created under it.
#[track_caller]
pub fn provide<T: Clone + 'static>(value: T) {
    let location = Location::caller();
    let value: Rc<dyn Any> = Rc::new(value);
    let replaced = with_rt(|rt| {
        let owner = rt.scoped_owner("provide()", location);
        let values = rt
            .owners
            .get_mut(owner)?
            .context
            .get_or_insert_with(Box::default);
        if let Some(held) = values.iter_mut().find(|held| held.is::<T>()) {
            Some(core::mem::replace(held, value))
        } else {
            values.push(value);
            None
        }
    });
    drop(replaced);
}

/// Returns the value of type `T` that the nearest enclosing owner provides.
#[must_use]
#[track_caller]
pub fn context<T: Clone + 'static>() -> Option<T> {
    let location = Location::caller();
    let held = with_rt(|rt| {
        let start = rt
            .lookup_owner()
            .unwrap_or_else(|| no_scope("context()", location));
        rt.provided::<T>(start)
    })?;
    held.downcast_ref::<T>().cloned()
}

impl RuntimeState {
    fn lookup_owner(&self) -> Option<OwnerKey> {
        self.settled_owner().or_else(|| {
            self.lazy_effect_owner
                .and_then(|key| self.effects.get(key))
                .map(|slot| slot.owner)
        })
    }

    fn provided<T: 'static>(&self, mut at: OwnerKey) -> Option<Rc<dyn Any>> {
        loop {
            let slot = self.owners.get(at)?;
            let mut values = slot.context.iter().flat_map(|values| values.iter());
            if let Some(held) = values.find(|held| held.is::<T>()) {
                return Some(Rc::clone(held));
            }
            at = slot.parent?;
        }
    }
}
