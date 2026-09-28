mod chrome;
mod event;
#[cfg(all(feature = "scene", not(target_arch = "wasm32")))]
pub(crate) mod gpu;
mod key;
mod report;
mod span;
mod store;

pub use key::{
    BackendKind, BackendStatus, Category, Cause, Counter, CounterValue, FallbackCause, Phase,
    PhaseTime,
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

pub fn gauge(counter: Counter, value: u64) {
    store::gauge(counter, value);
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
#[path = "profile/lib_tests.rs"]
mod tests;
