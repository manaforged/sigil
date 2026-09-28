#![cfg(target_arch = "wasm32")]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sigil::hal::Scheduler;
use sigil::reactive::Owner;
use sigil::tasks::{AsyncRuntime, OwnerHandle, sleep};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn a_spawned_task_runs_and_sleeps_on_the_browser_timer() {
    let runtime = AsyncRuntime::new().expect("runtime");
    let owner = Owner::new("tasks-wasm", None);
    let ran = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ran);
    let task = runtime.spawn(OwnerHandle::from_id(owner.id()), async move {
        sleep(Duration::from_millis(5)).await;
        flag.store(true, Ordering::SeqCst);
    });
    sleep(Duration::from_millis(50)).await;
    assert!(ran.load(Ordering::SeqCst), "the spawned task must run");
    assert!(task.is_finished(), "the task handle must report completion");
    owner.dispose();
}
