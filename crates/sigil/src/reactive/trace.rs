use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static TRACE_ENABLED: AtomicBool = AtomicBool::new(false);

static ENV_CHECKED: OnceLock<()> = OnceLock::new();

fn ensure_env_checked() {
    ENV_CHECKED.get_or_init(|| {
        let sigil_enabled =
            std::env::var_os("SIGIL_TRACE_EFFECTS").as_deref() == Some("1".as_ref());
        let legacy_enabled =
            std::env::var_os("RUST_UI_TRACE_EFFECTS").as_deref() == Some("1".as_ref());
        if sigil_enabled || legacy_enabled {
            TRACE_ENABLED.store(true, Ordering::Relaxed);
        }
    });
}

#[inline]
pub(crate) fn trace_active() -> bool {
    ensure_env_checked();
    if TRACE_ENABLED.load(Ordering::Relaxed) {
        return true;
    }
    test_hook::capture_enabled()
}

pub(crate) fn trace_effect_fire(
    name: &'static str,
    location: &'static core::panic::Location<'static>,
    reads_count: usize,
    writes_pending_count: usize,
) {
    if TRACE_ENABLED.load(Ordering::Relaxed) {
        eprintln!(
            "[sigil trace] effect fired: name={name} at={file}:{line} reads=[{reads_count}] writes_pending=[{writes_pending_count}]",
            file = location.file(),
            line = location.line(),
        );
    }

    test_hook::maybe_record(name, location, reads_count, writes_pending_count);
}

#[doc(hidden)]
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

    pub(crate) fn maybe_record(
        name: &'static str,
        location: &'static core::panic::Location<'static>,
        reads_count: usize,
        writes_pending_count: usize,
    ) {
        if !CAPTURE_ENABLED.with(Cell::get) {
            return;
        }
        let line = format!(
            "[sigil trace] effect fired: name={name} at={file}:{line} reads=[{reads_count}] writes_pending=[{writes_pending_count}]",
            file = location.file(),
            line = location.line(),
        );
        LAST_TRACE.with(|slot| *slot.borrow_mut() = Some(line));
    }
}
