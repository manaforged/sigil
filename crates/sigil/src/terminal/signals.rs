use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

static HALTED: AtomicBool = AtomicBool::new(false);
static PAINTING: Mutex<()> = Mutex::new(());

pub(crate) fn halted() -> bool {
    HALTED.load(Ordering::Relaxed)
}

pub(crate) fn painting() -> MutexGuard<'static, ()> {
    PAINTING.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn resume() {
    HALTED.store(false, Ordering::Relaxed);
}

#[cfg(unix)]
pub(crate) fn halt() {
    let _painting = painting();
    HALTED.store(true, Ordering::Relaxed);
    crate::terminal::term::restore();
}

#[cfg(unix)]
pub(crate) struct SignalGuard {
    handle: signal_hook::iterator::Handle,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(unix)]
impl SignalGuard {
    pub(crate) fn install() -> std::io::Result<Self> {
        use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
        let mut signals = signal_hook::iterator::Signals::new([SIGTERM, SIGHUP, SIGINT])?;
        let handle = signals.handle();
        let thread = std::thread::Builder::new()
            .name("sigil-terminal-signals".to_owned())
            .spawn(move || {
                if let Some(signal) = signals.forever().next() {
                    halt();
                    drop(signal_hook::low_level::emulate_default_handler(signal));
                }
            })?;
        Ok(Self {
            handle,
            thread: Some(thread),
        })
    }
}

#[cfg(unix)]
impl Drop for SignalGuard {
    fn drop(&mut self) {
        self.handle.close();
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

#[cfg(not(unix))]
pub(crate) struct SignalGuard;

#[cfg(not(unix))]
impl SignalGuard {
    pub(crate) const fn install() -> std::io::Result<Self> {
        Ok(Self)
    }
}
