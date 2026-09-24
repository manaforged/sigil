use std::cell::{Cell, RefCell};
use std::io::{self, BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::style::Print;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle, disable_raw_mode,
    enable_raw_mode,
};
use crossterm::{cursor, execute, queue};

use crate::terminal::caps::{Caps, Cursor, Keyboard};
use crate::terminal::screen::Screen;

const BUFFER: usize = 64 * 1024;

static RESTORED: AtomicBool = AtomicBool::new(true);
static TITLED: AtomicBool = AtomicBool::new(false);
static KEYS_PUSHED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static LIVE: Cell<bool> = const { Cell::new(false) };
    static REDRAW: Cell<bool> = const { Cell::new(false) };
    static FOCUSED: Cell<bool> = const { Cell::new(true) };
    static MOUSE: Cell<bool> = const { Cell::new(true) };
    static TITLE: RefCell<Option<String>> = const { RefCell::new(None) };
    static CAPS: Cell<Option<Caps>> = const { Cell::new(None) };
}

pub(crate) struct Term {
    pub(crate) redraw: bool,
    caps: Caps,
    out: BufWriter<io::Stdout>,
}

impl Term {
    pub(crate) fn enter(title: &str) -> io::Result<Self> {
        TITLE.with(|slot| *slot.borrow_mut() = Some(title.to_owned()));
        let guard = EnterGuard;
        RESTORED.store(false, Ordering::Relaxed);
        enable_raw_mode()?;
        let caps = Caps::probe();
        CAPS.with(|slot| slot.set(Some(caps)));
        enter_modes()?;
        core::mem::forget(guard);
        LIVE.with(|live| live.set(true));
        FOCUSED.with(|focused| focused.set(true));
        Ok(Self {
            redraw: true,
            caps,
            out: BufWriter::with_capacity(BUFFER, io::stdout()),
        })
    }

    pub(crate) fn paint(&mut self, screen: &mut Screen) -> io::Result<()> {
        let redraw = core::mem::take(&mut self.redraw) | REDRAW.with(|flag| flag.replace(false));
        paint_frame(&mut self.out, screen, redraw)
    }

    pub(crate) fn pending(&self) -> bool {
        self.redraw || REDRAW.with(Cell::get)
    }

    pub(crate) const fn caps(&self) -> Caps {
        self.caps
    }

    pub(crate) fn reenter(&mut self) -> io::Result<()> {
        enter_modes()?;
        self.redraw = true;
        Ok(())
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        LIVE.with(|live| live.set(false));
        restore();
        CAPS.with(|slot| slot.set(None));
        TITLE.with(|slot| drop(slot.borrow_mut().take()));
    }
}

fn enter_modes() -> io::Result<()> {
    RESTORED.store(false, Ordering::Relaxed);
    enable_raw_mode()?;
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        EnableBracketedPaste,
        EnableFocusChange,
        cursor::Hide
    )?;
    write_mouse(MOUSE.with(Cell::get))?;
    if current_caps().keyboard == Keyboard::Kitty {
        KEYS_PUSHED.store(true, Ordering::Relaxed);
        drop(execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                    | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
            )
        ));
    }
    TITLE.with(|slot| {
        slot.borrow()
            .as_deref()
            .filter(|title| !title.is_empty())
            .map_or(Ok(()), write_title)
    })
}

fn current_caps() -> Caps {
    CAPS.with(Cell::get).unwrap_or_default()
}

fn write_title(title: &str) -> io::Result<()> {
    TITLED.store(true, Ordering::Relaxed);
    execute!(io::stdout(), SetTitle(title))
}

pub(crate) fn paint_frame(
    out: &mut impl Write,
    screen: &mut Screen,
    redraw: bool,
) -> io::Result<()> {
    if !redraw && !screen.needs_flush() {
        return Ok(());
    }
    let sync = screen.caps().sync;
    if sync {
        queue!(out, Print("\x1b[?2026h"))?;
    }
    if redraw {
        queue!(out, Clear(ClearType::All))?;
        screen.invalidate();
    }
    screen.flush(out)?;
    if sync {
        queue!(out, Print("\x1b[?2026l"))?;
    }
    out.flush()
}

pub(crate) fn restore() {
    if RESTORED.swap(true, Ordering::Relaxed) {
        return;
    }
    if KEYS_PUSHED.swap(false, Ordering::Relaxed) {
        drop(execute!(io::stdout(), PopKeyboardEnhancementFlags));
    }
    if TITLED.swap(false, Ordering::Relaxed) {
        drop(execute!(io::stdout(), SetTitle("")));
    }
    if current_caps().cursor == Cursor::Bar {
        drop(execute!(
            io::stdout(),
            cursor::SetCursorStyle::DefaultUserShape
        ));
    }
    drop(execute!(
        io::stdout(),
        Print("\x1b[?2026l"),
        DisableBracketedPaste,
        DisableFocusChange,
        DisableMouseCapture,
        cursor::Show,
        LeaveAlternateScreen
    ));
    drop(disable_raw_mode());
}

pub(crate) fn set_focused(yes: bool) {
    FOCUSED.with(|focused| focused.set(yes));
}

pub fn suspend<R>(run: impl FnOnce() -> R) -> io::Result<R> {
    if !LIVE.with(Cell::get) {
        return Ok(run());
    }
    restore();
    let result = run();
    enter_modes()?;
    REDRAW.with(|flag| flag.set(true));
    Ok(result)
}

pub fn set_title(title: &str) -> io::Result<()> {
    TITLE.with(|slot| *slot.borrow_mut() = Some(title.to_owned()));
    if LIVE.with(Cell::get) {
        write_title(title)
    } else {
        Ok(())
    }
}

pub fn bell(note: Option<&str>) -> io::Result<()> {
    if !LIVE.with(Cell::get) {
        return Ok(());
    }
    let mut out = io::stdout().lock();
    if let Some(note) = note {
        let seq = format!("\x1b]9;{note}\x07");
        crate::terminal::graphics::write_mux(&mut out, &seq, current_caps().mux)?;
    }
    out.write_all(b"\x07")?;
    out.flush()
}

pub fn set_mouse_capture(yes: bool) -> io::Result<()> {
    MOUSE.with(|mouse| mouse.set(yes));
    if LIVE.with(Cell::get) {
        write_mouse(yes)
    } else {
        Ok(())
    }
}

fn write_mouse(yes: bool) -> io::Result<()> {
    if yes {
        execute!(io::stdout(), EnableMouseCapture)
    } else {
        execute!(io::stdout(), DisableMouseCapture)
    }
}

#[must_use]
pub fn focused() -> bool {
    FOCUSED.with(Cell::get)
}

struct EnterGuard;

impl Drop for EnterGuard {
    fn drop(&mut self) {
        restore();
    }
}

type PanicHook = Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync + 'static>;

pub(crate) struct HookGuard {
    previous: Option<PanicHook>,
}

impl HookGuard {
    pub(crate) fn install() -> Self {
        let previous: std::sync::Arc<PanicHook> = std::sync::Arc::new(std::panic::take_hook());
        let chained = std::sync::Arc::clone(&previous);
        std::panic::set_hook(Box::new(move |info| {
            restore();
            chained(info);
        }));
        Self {
            previous: Some(Box::new(move |info| previous(info))),
        }
    }
}

impl Drop for HookGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            std::panic::set_hook(previous);
        }
    }
}
