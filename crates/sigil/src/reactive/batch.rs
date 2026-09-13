use crate::reactive::effect::drain_pending;
use crate::reactive::runtime::with_rt;

struct BatchGuard;

impl Drop for BatchGuard {
    fn drop(&mut self) {
        with_rt(|rt| rt.batch_depth -= 1);
    }
}

pub fn batch<R>(f: impl FnOnce() -> R) -> R {
    with_rt(|rt| rt.batch_depth += 1);
    let guard = BatchGuard;
    let result = f();
    let is_outermost = with_rt(|rt| rt.batch_depth == 1);
    if is_outermost {
        drain_pending();
    }
    drop(guard);
    result
}
