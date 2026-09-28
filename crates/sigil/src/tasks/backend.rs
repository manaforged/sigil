use std::future::Future;

use crate::reactive::OwnerId;
use crate::tasks::task::{Guarded, Outcome, TaskHandle};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) type OwnedRuntime = Option<tokio::runtime::Runtime>;
#[cfg(target_arch = "wasm32")]
pub(crate) type OwnedRuntime = ();

#[derive(Clone)]
pub(crate) struct Backend {
    #[cfg(not(target_arch = "wasm32"))]
    handle: tokio::runtime::Handle,
}

impl Backend {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn new() -> std::io::Result<(Self, OwnedRuntime)> {
        use tokio::runtime::{Builder, Handle, RuntimeFlavor};
        match Handle::try_current() {
            Ok(handle) if handle.runtime_flavor() == RuntimeFlavor::MultiThread => {
                Ok((Self { handle }, None))
            }
            _ => {
                let runtime = Builder::new_multi_thread().enable_all().build()?;
                let handle = runtime.handle().clone();
                Ok((Self { handle }, Some(runtime)))
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) const fn new() -> std::io::Result<(Self, OwnedRuntime)> {
        Ok((Self {}, ()))
    }

    pub(crate) fn spawn<T, F, C>(&self, owner: OwnerId, future: F, on_done: C) -> TaskHandle
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
        C: FnOnce(Outcome<T>) + Send + 'static,
    {
        let task = TaskHandle::new();
        let done = task.clone();
        self.run(task.guard(future), move |outcome| {
            if let Outcome::Panicked(message) = &outcome {
                tracing::warn!(owner = ?owner, %message, "spawned task panicked");
            }
            done.finish();
            on_done(outcome);
        });
        task
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run<T, F, C>(&self, future: Guarded<F>, on_done: C)
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
        C: FnOnce(Outcome<T>) + Send + 'static,
    {
        let work = self.handle.spawn(future);
        drop(self.handle.spawn(async move {
            let outcome = match work.await {
                Ok(Some(value)) => Outcome::Done(value),
                Ok(None) => Outcome::Cancelled,
                Err(join) if join.is_cancelled() => Outcome::Cancelled,
                Err(join) => Outcome::Panicked(match join.try_into_panic() {
                    Ok(payload) => super::panic_message(&*payload).to_owned(),
                    Err(join) => join.to_string(),
                }),
            };
            on_done(outcome);
        }));
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(clippy::unused_self)]
    fn run<T, F, C>(&self, future: Guarded<F>, on_done: C)
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
        C: FnOnce(Outcome<T>) + Send + 'static,
    {
        wasm_bindgen_futures::spawn_local(async move {
            on_done(future.await.map_or(Outcome::Cancelled, Outcome::Done));
        });
    }
}
