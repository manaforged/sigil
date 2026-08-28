mod chrome;
mod event;
mod report;
mod span;
mod store;

pub use span::Span;
pub use store::{FRAME_BUDGET_NS, Frame, INTERACT_BUDGET_NS, Phase};

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

pub fn begin_frame() {
    store::begin_frame();
}

#[must_use]
pub fn span(name: &'static str) -> Span {
    Span::new(name)
}

pub fn count(name: &'static str) {
    store::count_add(name, 1);
}

pub fn count_add(name: &'static str, n: u64) {
    store::count_add(name, n);
}

pub fn interact(name: &'static str) {
    store::interact(name);
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
#[path = "lib_tests.rs"]
mod tests;
