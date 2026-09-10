#![allow(dead_code, unused_imports, unused_crate_dependencies)]

use sigil::prelude::*;

#[derive(Clone, DeriveAction)]
struct PreludeProbeAction;

fn _action_in_prelude() -> Box<dyn Action> {
    Box::new(PreludeProbeAction)
}

const fn _propagate_in_prelude(flag: &mut Propagate) {
    flag.propagate();
}

const fn _handler_key_in_prelude() -> HandlerKey {
    HandlerKey::from_raw(1)
}

#[test]
fn dispatch_to_owner_resolves_through_prelude() {
    let f: fn(&_, OwnerHandle, fn()) = dispatch_to_owner::<fn()>;
    let _ = f;
}

#[test]
fn spawn_resolves_through_prelude() {
    type Fut = std::future::Ready<Result<u32, &'static str>>;
    #[allow(clippy::type_complexity)]
    let _: fn(Fut) -> Signal<Status<u32, &'static str>> = spawn;
    let _ = Status::<u32, &'static str>::Pending;
}
