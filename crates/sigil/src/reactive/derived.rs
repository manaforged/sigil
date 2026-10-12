use core::panic::Location;
use std::cell::Cell;
use std::rc::Rc;

use crate::reactive::diagnostic::no_scope;
use crate::reactive::effect::effect;
use crate::reactive::runtime::with_rt;
use crate::reactive::signal::{Signal, signal_in};

#[must_use]
#[track_caller]
pub fn derived<T, F>(mut f: F) -> Signal<T>
where
    T: Clone + PartialEq + 'static,
    F: FnMut() -> T + 'static,
{
    let caller = Location::caller();
    let parent_owner =
        with_rt(|rt| rt.current_owner).unwrap_or_else(|| no_scope("derived()", caller));
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
