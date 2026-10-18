use std::cell::{Cell, RefCell};
use std::io::{self, BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread::ThreadId;

use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, EnableBracketedPaste, EnableFocusChange,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::style::Print;
use crossterm::terminal::{
    Clear, ClearType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen, LeaveAlternateScreen,
    SetTitle, disable_raw_mode, enable_raw_mode,
};
use crossterm::{cursor, execute, queue};

use crate::terminal::caps::{Caps, Cursor, Keyboard};
use crate::terminal::mouse::{MOUSE_OFF, MouseTracking};
use crate::terminal::screen::Screen;
use crate::terminal::signals;

mod above;
mod calibrate;

pub use above::{PrintAboveError, print_above};
pub(crate) use calibrate::Measured;
pub use calibrate::calibrate;
#[cfg(unix)]
pub(crate) use calibrate::{measure, query as calibration};

const BUFFER: usize = 64 * 1024;
const MODIFY_KEYS_ON: &str = "\x1b[>4;2m";
const MODIFY_KEYS_OFF: &str = "\x1b[>4m";

static RESTORED: AtomicBool = AtomicBool::new(true);
static TITLED: AtomicBool = AtomicBool::new(false);
static KEYS_PUSHED: AtomicBool = AtomicBool::new(false);
static MODIFY_KEYS_WANTED: AtomicBool = AtomicBool::new(false);
static MODIFY_KEYS_SET: AtomicBool = AtomicBool::new(false);
static INLINE: AtomicBool = AtomicBool::new(false);
static BAR_CURSOR: AtomicBool = AtomicBool::new(false);
static OWNER: Mutex<Option<ThreadId>> = Mutex::new(None);

thread_local! {
    static LIVE: Cell<bool> = const { Cell::new(false) };
    static REDRAW: Cell<bool> = const { Cell::new(false) };
    static FOCUSED: Cell<bool> = const { Cell::new(true) };
    static MOUSE: Cell<MouseTracking> = const { Cell::new(MouseTracking::Drags) };
    static TITLE: RefCell<Option<String>> = const { RefCell::new(None) };
    static CAPS: Cell<Option<Caps>> = const { Cell::new(None) };
    static LIVE_ROWS: Cell<u16> = const { Cell::new(0) };
}

pub(crate) struct Term {
    pub(crate) redraw: bool,
    inline: bool,
    caps: Caps,
    out: BufWriter<io::Stdout>,
}

impl Term {
    pub(crate) fn enter(config: &crate::AppConfig) -> io::Result<Self> {
        let inline = config.inline;
        MODIFY_KEYS_WANTED.store(
            config.other_keys == crate::app::config::OtherKeys::Modified,
            Ordering::Relaxed,
        );
        TITLE.with(|slot| *slot.borrow_mut() = Some(config.title.clone()));
        set_owner(Some(std::thread::current().id()));
        let guard = EnterGuard;
        RESTORED.store(false, Ordering::Relaxed);
        enable_raw_mode()?;
        enter_screen(inline)?;
        let (caps, measured) = Caps::measure(!inline);
        calibrate::keep(measured);
        CAPS.with(|slot| slot.set(Some(caps)));
        BAR_CURSOR.store(caps.cursor == Cursor::Bar, Ordering::Relaxed);
        enter_modes(inline)?;
        core::mem::forget(guard);
        LIVE.with(|live| live.set(true));
        FOCUSED.with(|focused| focused.set(true));
        let mut term = Self {
            redraw: true,
            inline,
            caps,
            out: BufWriter::with_capacity(BUFFER, io::stdout()),
        };
        if inline {
            ground(&mut term.out)?;
        }
        Ok(term)
    }

    pub(crate) fn paint(&mut self, screen: &mut Screen) -> io::Result<()> {
        let _painting = signals::painting();
        if signals::halted() {
            return Ok(());
        }
        if signals::take_resumed() {
            self.reenter()?;
        }
        if signals::suspended() {
            return Ok(());
        }
        let redraw = core::mem::take(&mut self.redraw) | REDRAW.with(|flag| flag.replace(false));
        paint_frame(&mut self.out, screen, redraw, self.inline)
    }

    pub(crate) fn pending(&self) -> bool {
        self.redraw || REDRAW.with(Cell::get) || signals::resumed()
    }

    pub(crate) const fn caps(&self) -> Caps {
        self.caps
    }

    pub(crate) fn reenter(&mut self) -> io::Result<()> {
        enter_screen(self.inline)?;
        enter_modes(self.inline)?;
        if self.inline {
            ground(&mut self.out)?;
        }
        self.redraw = true;
        Ok(())
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        LIVE.with(|live| live.set(false));
        restore();
        if self.inline {
            drop(execute!(io::stdout(), Print("\n")));
        }
        CAPS.with(|slot| slot.set(None));
        TITLE.with(|slot| drop(slot.borrow_mut().take()));
        set_owner(None);
    }
}

fn set_owner(owner: Option<ThreadId>) {
    *OWNER.lock().unwrap_or_else(PoisonError::into_inner) = owner;
}

fn owns_terminal() -> bool {
    OWNER
        .try_lock()
        .ok()
        .and_then(|owner| *owner)
        .is_some_and(|owner| owner == std::thread::current().id())
}

fn keyboard_flags() -> KeyboardEnhancementFlags {
    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
        | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
}

fn enter_screen(inline: bool) -> io::Result<()> {
    INLINE.store(inline, Ordering::Relaxed);
    if !inline {
        execute!(io::stdout(), EnterAlternateScreen, DisableLineWrap)?;
    }
    Ok(())
}

fn enter_modes(inline: bool) -> io::Result<()> {
    INLINE.store(inline, Ordering::Relaxed);
    RESTORED.store(false, Ordering::Relaxed);
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, EnableBracketedPaste, EnableFocusChange, cursor::Hide)?;
    write_mouse(&mut out, MOUSE.with(Cell::get))?;
    let keyboard = current_caps().keyboard;
    if keyboard == Keyboard::Kitty {
        KEYS_PUSHED.store(true, Ordering::Relaxed);
        drop(execute!(
            out,
            PushKeyboardEnhancementFlags(keyboard_flags())
        ));
    }
    let modify = modify_keys_sequence(keyboard, MODIFY_KEYS_WANTED.load(Ordering::Relaxed));
    if !modify.is_empty() {
        MODIFY_KEYS_SET.store(true, Ordering::Relaxed);
        drop(execute!(out, Print(modify)));
    }
    TITLE.with(|slot| {
        slot.borrow()
            .as_deref()
            .filter(|title| !title.is_empty())
            .map_or(Ok(()), write_title)
    })
}

fn modify_keys_sequence(keyboard: Keyboard, wanted: bool) -> &'static str {
    if wanted && keyboard != Keyboard::Kitty {
        MODIFY_KEYS_ON
    } else {
        ""
    }
}

#[must_use]
pub fn caps() -> Option<Caps> {
    CAPS.with(Cell::get)
}

pub(crate) fn current_caps() -> Caps {
    caps().unwrap_or_default()
}

fn write_title(title: &str) -> io::Result<()> {
    TITLED.store(true, Ordering::Relaxed);
    execute!(io::stdout(), SetTitle(title))
}

fn ground(out: &mut impl Write) -> io::Result<()> {
    queue!(out, cursor::MoveToColumn(0))?;
    let Ok((_, row)) = cursor::position() else {
        return out.write_all(b"\n").and_then(|()| out.flush());
    };
    let rows = crossterm::terminal::size().map_or(0, |(_, rows)| rows);
    for _ in row..rows.saturating_sub(1) {
        queue!(out, Print("\n"))?;
    }
    out.flush()
}

pub(crate) fn paint_frame(
    out: &mut impl Write,
    screen: &mut Screen,
    redraw: bool,
    inline: bool,
) -> io::Result<()> {
    if !redraw && !screen.needs_flush() {
        return Ok(());
    }
    let sync = screen.caps().sync;
    if sync {
        queue!(out, Print("\x1b[?2026h"))?;
    }
    if redraw && !inline {
        queue!(out, Clear(ClearType::All))?;
    }
    if redraw {
        screen.invalidate();
    }
    if inline {
        screen.flush_inline(out)?;
        LIVE_ROWS.with(|rows| rows.set(u16::try_from(screen.rows()).unwrap_or(u16::MAX)));
    } else {
        screen.flush(out)?;
    }
    if sync {
        queue!(out, Print("\x1b[?2026l"))?;
    }
    out.flush()
}

pub(crate) fn restore() {
    if RESTORED.swap(true, Ordering::Relaxed) {
        return;
    }
    let mut out = io::stdout();
    if KEYS_PUSHED.swap(false, Ordering::Relaxed) {
        drop(execute!(out, PopKeyboardEnhancementFlags));
    }
    if MODIFY_KEYS_SET.swap(false, Ordering::Relaxed) {
        drop(execute!(out, Print(MODIFY_KEYS_OFF)));
    }
    if TITLED.swap(false, Ordering::Relaxed) {
        drop(execute!(out, SetTitle("")));
    }
    if BAR_CURSOR.load(Ordering::Relaxed) {
        drop(execute!(out, cursor::SetCursorStyle::DefaultUserShape));
    }
    drop(execute!(
        out,
        Print("\x1b[?2026l"),
        DisableBracketedPaste,
        DisableFocusChange,
        Print(MOUSE_OFF),
        cursor::Show
    ));
    if !INLINE.swap(false, Ordering::Relaxed) {
        drop(execute!(out, LeaveAlternateScreen, EnableLineWrap));
    }
    drop(disable_raw_mode());
}

pub(crate) fn set_focused(yes: bool) {
    FOCUSED.with(|focused| focused.set(yes));
}

pub fn suspend<R>(run: impl FnOnce() -> R) -> io::Result<R> {
    if !live() {
        return Ok(run());
    }
    let inline = INLINE.load(Ordering::Relaxed);
    restore();
    let result = run();
    enter_screen(inline)?;
    enter_modes(inline)?;
    if inline {
        ground(&mut io::stdout())?;
    }
    REDRAW.with(|flag| flag.set(true));
    Ok(result)
}

pub fn set_title(title: &str) -> io::Result<()> {
    TITLE.with(|slot| *slot.borrow_mut() = Some(title.to_owned()));
    if live() {
        write_title(title)?;
    }
    Ok(())
}

pub(crate) fn live() -> bool {
    LIVE.with(Cell::get)
}

pub fn set_mouse_capture(yes: bool) -> io::Result<()> {
    set_mouse_tracking(if yes {
        MouseTracking::Drags
    } else {
        MouseTracking::Off
    })
}

pub fn set_mouse_tracking(tracking: MouseTracking) -> io::Result<()> {
    MOUSE.with(|mouse| mouse.set(tracking));
    if live() {
        write_mouse(&mut io::stdout(), tracking)?;
    }
    Ok(())
}

fn write_mouse(out: &mut impl Write, tracking: MouseTracking) -> io::Result<()> {
    out.write_all(MOUSE_OFF.as_bytes())?;
    out.write_all(tracking.sequence().as_bytes())?;
    out.flush()
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
            if owns_terminal() {
                restore();
            }
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

#[cfg(test)]
#[path = "term_tests.rs"]
mod tests;
