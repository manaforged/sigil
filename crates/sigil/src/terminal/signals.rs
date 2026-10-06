use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

static HALTED: AtomicBool = AtomicBool::new(false);
static SUSPENDED: AtomicBool = AtomicBool::new(false);
static RESUMED: AtomicBool = AtomicBool::new(false);
static PAINTING: Mutex<()> = Mutex::new(());
#[cfg(unix)]
static WAKE: Mutex<Option<std::os::unix::net::UnixStream>> = Mutex::new(None);

pub(crate) fn halted() -> bool {
    HALTED.load(Ordering::Relaxed)
}

pub(crate) fn stopped() -> bool {
    crate::app::take_exit() || halted()
}

pub(crate) fn painting() -> MutexGuard<'static, ()> {
    PAINTING.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn resume() {
    HALTED.store(false, Ordering::Relaxed);
}

pub(crate) fn suspended() -> bool {
    SUSPENDED.load(Ordering::Relaxed)
}

pub(crate) fn resumed() -> bool {
    RESUMED.load(Ordering::Relaxed)
}

pub(crate) fn take_resumed() -> bool {
    RESUMED.swap(false, Ordering::Relaxed)
}

#[cfg(unix)]
pub(crate) fn set_wake(wake: Option<std::os::unix::net::UnixStream>) {
    *WAKE.lock().unwrap_or_else(PoisonError::into_inner) = wake;
}

#[cfg(unix)]
fn wake() {
    use std::io::Write;
    if let Some(wake) = WAKE.lock().unwrap_or_else(PoisonError::into_inner).as_ref() {
        drop((&*wake).write(&[1]));
    }
}

#[cfg(unix)]
pub(crate) fn stop() {
    drop(signal_hook::low_level::raise(signal_hook::consts::SIGTSTP));
}

#[cfg(not(unix))]
pub(crate) const fn stop() {}

#[cfg(unix)]
fn pause() {
    {
        let _painting = painting();
        SUSPENDED.store(true, Ordering::Relaxed);
        crate::terminal::term::restore();
    }
    drop(signal_hook::low_level::emulate_default_handler(
        signal_hook::consts::SIGTSTP,
    ));
}

#[cfg(unix)]
fn proceed() {
    SUSPENDED.store(false, Ordering::Relaxed);
    RESUMED.store(true, Ordering::Relaxed);
    wake();
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
        use signal_hook::consts::{SIGCONT, SIGHUP, SIGINT, SIGTERM, SIGTSTP};
        let mut signals =
            signal_hook::iterator::Signals::new([SIGTERM, SIGHUP, SIGINT, SIGTSTP, SIGCONT])?;
        let handle = signals.handle();
        let thread = std::thread::Builder::new()
            .name("sigil-terminal-signals".to_owned())
            .spawn(move || {
                for signal in signals.forever() {
                    match signal {
                        SIGTSTP => pause(),
                        SIGCONT => proceed(),
                        _ => {
                            halt();
                            drop(signal_hook::low_level::emulate_default_handler(signal));
                            break;
                        }
                    }
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
        set_wake(None);
        SUSPENDED.store(false, Ordering::Relaxed);
        RESUMED.store(false, Ordering::Relaxed);
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
