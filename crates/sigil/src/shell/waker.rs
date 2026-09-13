use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use winit::event_loop::EventLoopProxy;

use crate::shell::ShellError;

#[doc(hidden)]
pub static WAKE_DELIVERED_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct WakeEvent;

#[derive(Clone)]
pub struct WinitWaker {
    pub(crate) proxy: Arc<Mutex<EventLoopProxy<WakeEvent>>>,
    pub(crate) pending: Arc<AtomicBool>,
    pub(crate) alive: Arc<AtomicBool>,
}

impl WinitWaker {
    pub fn wake(&self) -> Result<(), ShellError> {
        if !self.alive.load(Ordering::Acquire) {
            return Err(ShellError::EventLoopClosed);
        }

        if self
            .pending
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Ok(());
        }

        let proxy = self.proxy.lock().map_err(|_| ShellError::WakerPoisoned)?;
        if proxy.send_event(WakeEvent).is_ok() {
            WAKE_DELIVERED_COUNT.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            self.pending.store(false, Ordering::Release);
            self.alive.store(false, Ordering::Release);
            Err(ShellError::EventLoopClosed)
        }
    }
}

impl core::fmt::Debug for WinitWaker {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WinitWaker").finish_non_exhaustive()
    }
}
