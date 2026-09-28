use core::panic::Location;
use std::cell::Cell;
use std::rc::Rc;

use smallvec::SmallVec;

use crate::reactive::effect::effect;
use crate::reactive::runtime::{OwnerKey, SignalSlot, with_rt};
use crate::reactive::signal::Signal;

fn signal_in_owner<T: 'static>(
    initial: T,
    owner: OwnerKey,
    location: &'static Location<'static>,
) -> Signal<T> {
    let key = with_rt(|rt| {
        assert!(
            rt.owners.contains(owner),
            "derived signal parent owner was disposed (at {location})",
        );
        let key = rt.signals.insert(SignalSlot {
            owner,
            value: Box::new(initial),
            subscribers: SmallVec::new(),
            location,
        });
        rt.owners
            .get_mut(owner)
            .expect("owner validated above")
            .signals
            .push(key);
        key
    });
    Signal::from_key(key)
}

fn current_owner_or_panic(caller: &'static Location<'static>) -> OwnerKey {
    with_rt(|rt| {
        rt.current_owner
            .unwrap_or_else(|| crate::reactive::diagnostic::no_scope("derived()", caller))
    })
}

#[must_use]
#[track_caller]
pub fn derived<T, F>(mut f: F) -> Signal<T>
where
    T: Clone + PartialEq + 'static,
    F: FnMut() -> T + 'static,
{
    let caller = Location::caller();
    let parent_owner = current_owner_or_panic(caller);

    let holder: Rc<Cell<Option<Signal<T>>>> = Rc::new(Cell::new(None));
    let holder_inside = Rc::clone(&holder);

    effect(move || {
        let value = f();
        match holder_inside.get() {
            None => {
                let sig = signal_in_owner(value, parent_owner, caller);
                holder_inside.set(Some(sig));
            }
            Some(sig) => sig.set(value),
        }
    });

    holder.get().expect(
        "derived's tracking effect populates the backing signal on its synchronous first run",
    )
}
