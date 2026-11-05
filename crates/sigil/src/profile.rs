mod attr;
mod chrome;
mod event;
#[cfg(all(
    feature = "scene",
    not(target_arch = "wasm32"),
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
pub(crate) mod gpu;
#[cfg(any(test, feature = "desktop", feature = "kittest"))]
mod input;
mod key;
mod report;
mod report_why;
mod site;
mod span;
mod store;

pub(crate) use key::Hotspot;
pub(crate) use attr::budget_ns;
pub(crate) use chrome::site_name;
pub(crate) use site::site;
pub use site::{Site, SiteKind};
pub use key::{BackendKind, BackendStatus, Cause, Counter, FallbackCause, Phase};
kittest_pub! {
    #[doc(hidden)]
    pub use span::Span;
}
kittest_pub! {
    #[doc(hidden)]
    pub use store::Frame;
}
kittest_pub! {
    #[doc(hidden)]
    pub use store::FRAME_BUDGET_NS;
}

/// Returns whether profiling is on.
#[must_use]
pub(crate) fn enabled() -> bool {
    store::enabled()
}

/// Turns profiling on.
pub fn enable() {
    store::enable();
}

/// Turns profiling off.
pub fn disable() {
    store::disable();
}

/// Clears the recorded profile data.
pub fn reset() {
    store::reset();
}

/// Starts a profiled frame on `backend`.
pub fn begin_frame(backend: BackendStatus) {
    store::begin_frame(backend);
}

/// Starts timing `phase` in the current frame.
#[must_use]
pub fn span(phase: Phase) -> Span {
    Span::new(phase)
}

/// Adds one to `counter`.
pub(crate) fn count(counter: Counter) {
    store::count_add(counter, 1);
}

/// Adds `n` to `counter`.
pub fn count_add(counter: Counter, n: u64) {
    store::count_add(counter, n);
}

pub(crate) use attr::attribute;
#[cfg(any(
    test,
    all(
        feature = "scene",
        any(feature = "desktop", feature = "kittest", feature = "terminal")
    )
))]
pub(crate) use attr::gpu_pass;

/// Records the current value of a gauge.
#[cfg(feature = "scene")]
pub(crate) fn gauge(counter: Counter, value: u64) {
    store::gauge(counter, value);
}

#[cfg(any(test, feature = "desktop", feature = "kittest"))]
kittest_pub! {
    #[doc(hidden)]
    pub use input::note_input;
}

/// Marks the next frame as caused by `cause` so the report can measure input latency.
pub fn interact(cause: Cause) {
    store::interact(cause);
}

/// Ends the current profiled frame.
pub fn tick_frame() {
    store::tick_frame();
}

/// Writes the recorded profile to `path`.
pub fn dump(path: impl AsRef<std::path::Path>) -> Result<(), crate::Error> {
    store::close_open();
    let path = path.as_ref();
    chrome::write(path)?;
    eprintln!("{}", report::summary());
    eprintln!("sigil-profile: wrote {}", path.display());
    Ok(())
}

#[cfg(all(feature = "inspect", any(feature = "desktop", feature = "terminal")))]
pub(crate) fn dump_path() -> Option<std::path::PathBuf> {
    store::dump_path()
}

#[cfg(any(feature = "desktop", feature = "terminal"))]
pub(crate) fn dump_if_enabled() {
    if let Some(path) = store::dump_path()
        && let Err(err) = dump(&path)
    {
        eprintln!("sigil-profile: dump failed: {err}");
    }
}

/// Returns the recorded frames.
#[must_use]
pub fn frames() -> Vec<Frame> {
    store::frames()
}

kittest_pub! {
    #[cfg(any(
        test,
        feature = "kittest",
        all(feature = "inspect", any(feature = "desktop", feature = "terminal"))
    ))]
    #[doc(hidden)]
    #[must_use]
    pub fn summary() -> String {
        report::summary()
    }
}

#[cfg(test)]
pub(crate) fn exclusive() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
#[path = "profile/lib_tests.rs"]
mod tests;
