mod attr;
mod chrome;
mod event;
#[cfg(all(feature = "scene", not(target_arch = "wasm32")))]
pub(crate) mod gpu;
mod input;
mod key;
mod report;
mod report_why;
mod span;
mod store;

pub use attr::{Attribution, BUDGET_ENV, DEFAULT_BUDGET_MS, MAX_SUBJECTS, OTHER_SUBJECT};
pub use key::{
    BackendKind, BackendStatus, Category, Cause, Counter, CounterValue, FallbackCause, Hotspot,
    Phase, PhaseTime,
};
pub use span::Span;
pub use store::{FRAME_BUDGET_NS, Frame, INTERACT_BUDGET_NS};

#[must_use]
pub fn enabled() -> bool {
    store::enabled()
}

pub fn enable() {
    store::enable();
}

pub fn disable() {
    store::disable();
}

pub fn reset() {
    store::reset();
}

pub fn begin_frame(backend: BackendStatus) {
    store::begin_frame(backend);
}

#[must_use]
pub fn span(phase: Phase) -> Span {
    Span::new(phase)
}

pub fn count(counter: Counter) {
    store::count_add(counter, 1);
}

pub fn count_add(counter: Counter, n: u64) {
    store::count_add(counter, n);
}

pub fn attribute(hotspot: Hotspot, n: u64, subject: impl FnOnce() -> String) {
    attr::attribute(hotspot, n, subject);
}

pub fn gpu_pass(label: &'static str, ns: u64) {
    attr::gpu_pass(label, ns);
}

#[must_use]
pub fn frame_budget_ns() -> u64 {
    attr::budget_ns()
}

pub fn gauge(counter: Counter, value: u64) {
    store::gauge(counter, value);
}

pub fn note_input(event: &crate::events::ShellEvent) {
    input::note_input(event);
}

pub fn interact(cause: Cause) {
    store::interact(cause);
}

pub fn tick_frame() {
    store::tick_frame();
}

pub fn dump(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
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

pub fn dump_if_enabled() {
    if let Some(path) = store::dump_path()
        && let Err(err) = dump(&path)
    {
        eprintln!("sigil-profile: dump failed: {err}");
    }
}

#[must_use]
pub fn frames() -> Vec<Frame> {
    store::frames()
}

#[must_use]
pub fn summary() -> String {
    report::summary()
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
