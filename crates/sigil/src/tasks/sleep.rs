use std::time::Duration;

/// Waits for `duration` inside a `task` or `spawn` future without blocking a thread.
pub async fn sleep(duration: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;
    #[cfg(target_arch = "wasm32")]
    crate::tasks::timer::browser_timer(duration).await.ok();
}
