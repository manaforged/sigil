#![allow(unused_crate_dependencies)]

use sigil::prelude::*;

#[test]
fn spawn_resolves_through_prelude() {
    type Fut = std::future::Ready<Result<u32, &'static str>>;
    #[allow(clippy::type_complexity)]
    let _: fn(Fut) -> Signal<Status<u32, &'static str>> = spawn;
    let _ = Status::<u32, &'static str>::Pending;
}
