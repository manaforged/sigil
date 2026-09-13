use core::error::Error;
use core::future::Future;

extern crate alloc;

use alloc::boxed::Box;

pub trait Scheduler {
    type Error: Error + Send + Sync + 'static;

    type OwnerHandle: Clone + Send + Sync + 'static;

    type SpawnTask: Send + 'static;

    fn enqueue_main(
        &self,
        owner: Self::OwnerHandle,
        f: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), Self::Error>;

    fn spawn<F>(&self, owner: Self::OwnerHandle, future: F) -> Self::SpawnTask
    where
        F: Future<Output = ()> + Send + 'static;

    fn drain(&mut self) -> usize;
}
