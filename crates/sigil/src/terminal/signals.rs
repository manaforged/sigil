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
                    crate::terminal::term::restore();
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
