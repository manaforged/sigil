#[cfg(not(target_arch = "wasm32"))]
pub async fn blocking<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return f();
    };
    match runtime.spawn_blocking(f).await {
        Ok(value) => value,
        Err(join) => match join.try_into_panic() {
            Ok(payload) => std::panic::resume_unwind(payload),
            Err(join) => {
                std::panic::resume_unwind(Box::new(format!("blocking work stopped: {join}")))
            }
        },
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::unused_async)]
pub async fn blocking<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    f()
}
