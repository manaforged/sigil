use crate::reactive::effect::drain_pending;
use crate::reactive::runtime::with_rt;

struct BatchGuard;

impl Drop for BatchGuard {
    fn drop(&mut self) {
        with_rt(|rt| rt.batch_depth = rt.batch_depth.saturating_sub(1));
    }
}

pub fn batch<R>(f: impl FnOnce() -> R) -> R {
    let depth = with_rt(|rt| {
        rt.batch_depth += 1;
        rt.batch_depth
    });
    let guard = BatchGuard;
    let result = f();
    if depth == 1 {
        drain_pending();
    }
    drop(guard);
    result
}
