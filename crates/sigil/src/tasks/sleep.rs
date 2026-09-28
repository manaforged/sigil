use std::time::Duration;

pub async fn sleep(duration: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;
    #[cfg(target_arch = "wasm32")]
    let _ = crate::tasks::timer::browser_timer(duration).await;
}
