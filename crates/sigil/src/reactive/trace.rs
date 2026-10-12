use std::sync::LazyLock;

static TRACE_ENABLED: LazyLock<bool> = LazyLock::new(|| {
    ["SIGIL_TRACE_EFFECTS", "RUST_UI_TRACE_EFFECTS"]
        .iter()
        .any(|var| std::env::var_os(var).as_deref() == Some("1".as_ref()))
});

#[inline]
pub(crate) fn trace_active() -> bool {
    *TRACE_ENABLED || test_hook::capture_enabled()
}

pub(crate) fn trace_effect_fire(
    name: &'static str,
    location: &'static core::panic::Location<'static>,
    reads_count: usize,
    writes_pending_count: usize,
) {
    let line = format!(
        "[sigil trace] effect fired: name={name} at={file}:{line} reads=[{reads_count}] writes_pending=[{writes_pending_count}]",
        file = location.file(),
        line = location.line(),
    );
    if *TRACE_ENABLED {
        eprintln!("{line}");
    }
    test_hook::maybe_record(&line);
}

#[cfg(any(test, feature = "kittest"))]
pub mod test_hook {
    use std::cell::{Cell, RefCell};

    thread_local! {
        static LAST_TRACE: RefCell<Option<String>> = const { RefCell::new(None) };
        static CAPTURE_ENABLED: Cell<bool> = const { Cell::new(false) };
    }

    pub fn enable_capture() {
        CAPTURE_ENABLED.with(|cell| cell.set(true));
    }

    pub(crate) fn capture_enabled() -> bool {
        CAPTURE_ENABLED.with(Cell::get)
    }

    pub fn disable_capture() {
        CAPTURE_ENABLED.with(|cell| cell.set(false));
        reset_last_line();
    }

    #[must_use]
    pub fn take_last_line() -> Option<String> {
        LAST_TRACE.with(|slot| slot.borrow_mut().take())
    }

    pub fn reset_last_line() {
        LAST_TRACE.with(|slot| *slot.borrow_mut() = None);
    }

    pub(crate) fn maybe_record(line: &str) {
        if capture_enabled() {
            LAST_TRACE.with(|slot| *slot.borrow_mut() = Some(line.to_owned()));
        }
    }
}

#[cfg(not(any(test, feature = "kittest")))]
mod test_hook {
    pub(crate) const fn capture_enabled() -> bool {
        false
    }

    pub(crate) const fn maybe_record(_: &str) {}
}
