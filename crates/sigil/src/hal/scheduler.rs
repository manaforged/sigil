use core::error::Error;
use core::future::Future;

extern crate alloc;

use alloc::boxed::Box;

pub trait Scheduler {
    type Error: Error + Send + Sync + 'static;

    type Owner: Clone + Send + Sync + 'static;

    type SpawnTask: Send + 'static;

    #[cfg(any(test, feature = "kittest"))]
    fn enqueue_main(
        &self,
        owner: Self::Owner,
        f: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), Self::Error>;

    #[cfg(any(test, feature = "kittest"))]
    fn spawn<F>(&self, owner: Self::Owner, future: F) -> Self::SpawnTask
    where
        F: Future<Output = ()> + Send + 'static;

    fn drain(&mut self) -> usize;
}
