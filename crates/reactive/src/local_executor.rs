use core::future::Future;
use core::pin::Pin;

use crate::owner::OwnerId;
use tokio::task::AbortHandle;

pub type BoxLocalFuture = Pin<Box<dyn Future<Output = ()> + 'static>>;

pub trait LocalTaskExecutor: 'static {
    fn spawn_local(&self, owner: OwnerId, fut: BoxLocalFuture) -> AbortHandle;
}
