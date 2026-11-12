use core::panic::Location;
use std::cell::Cell;
use std::rc::Rc;

use crate::reactive::diagnostic::no_scope;
use crate::reactive::effect::effect;
use crate::reactive::runtime::{RuntimeState, with_rt};
use crate::reactive::signal::{Signal, signal_in};

/// Creates a signal that holds the result of `f` and updates when a signal that `f` reads changes.
#[must_use]
#[track_caller]
pub fn memo<T, F>(mut f: F) -> Signal<T>
where
    T: Clone + PartialEq + 'static,
    F: FnMut() -> T + 'static,
{
    let caller = Location::caller();
    let parent_owner =
        with_rt(RuntimeState::active_owner).unwrap_or_else(|| no_scope("memo()", caller));
    let holder: Rc<Cell<Option<Signal<T>>>> = Rc::new(Cell::new(None));
    let holder_inside = Rc::clone(&holder);
    effect(move || {
        let value = f();
        match holder_inside.get() {
            None => holder_inside.set(Some(signal_in(value, parent_owner, caller))),
            Some(sig) => sig.set(value),
        }
    });
    holder.get().expect(
        "derived's tracking effect populates the backing signal on its synchronous first run",
    )
}
