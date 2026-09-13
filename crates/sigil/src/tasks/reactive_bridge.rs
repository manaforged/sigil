use std::sync::Arc;

use crate::reactive::{
    BoxMainCallback, BoxSendFuture, DispatchError, MainThreadDispatcher, OwnerId, Runtime,
    TaskExecutor,
};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::AbortHandle;

pub struct TokioExecutor {
    handle: Handle,
}

impl core::fmt::Debug for TokioExecutor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TokioExecutor").finish_non_exhaustive()
    }
}

impl TokioExecutor {
    #[must_use]
    pub const fn new(handle: Handle) -> Self {
        Self { handle }
    }
}

impl TaskExecutor for TokioExecutor {
    fn spawn(&self, _owner: OwnerId, fut: BoxSendFuture) -> AbortHandle {
        self.handle.spawn(fut).abort_handle()
    }
}

const _: fn() = || {
    const fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<TokioExecutor>();
};

#[derive(Clone)]
pub struct ReactiveBridge {
    tx: UnboundedSender<(OwnerId, BoxMainCallback)>,
    waker: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl core::fmt::Debug for ReactiveBridge {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ReactiveBridge").finish_non_exhaustive()
    }
}

impl MainThreadDispatcher for ReactiveBridge {
    fn post(&self, owner: OwnerId, callback: BoxMainCallback) -> Result<(), DispatchError> {
        self.tx
            .send((owner, callback))
            .map_err(|_send_err| DispatchError::Closed)?;
        (self.waker)();
        Ok(())
    }
}

const _: fn() = || {
    const fn assert_send_sync_clone_static<T: Send + Sync + Clone + 'static>() {}
    assert_send_sync_clone_static::<ReactiveBridge>();
};

pub(crate) fn build_bridge() -> (
    ReactiveBridge,
    UnboundedReceiver<(OwnerId, BoxMainCallback)>,
) {
    let (tx, rx) = unbounded_channel();
    let waker: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(|| {});
    (ReactiveBridge { tx, waker }, rx)
}

pub(crate) fn drain_into_reactive(rx: &mut UnboundedReceiver<(OwnerId, BoxMainCallback)>) -> usize {
    let budget = rx.len();
    let mut moved = 0_usize;
    for _ in 0..budget {
        let Ok((owner, callback)) = rx.try_recv() else {
            break;
        };
        Runtime::enqueue_completion(owner, callback);
        moved += 1;
    }
    moved
}

pub(crate) fn replace_bridge_waker(
    bridge: &mut ReactiveBridge,
    waker: Arc<dyn Fn() + Send + Sync + 'static>,
) {
    bridge.waker = waker;
}
