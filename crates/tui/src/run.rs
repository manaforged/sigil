use std::io::{self, BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::style::Print;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{cursor, execute, queue};
use sigil::app::{
    AppConfig, AppResult, CellWidth, Error, FrameHooks, Host, LayoutError, PanicPolicy, TreeRoot,
    set_cell_width,
};
use sigil::events::EventRouter;
use sigil::hal::ShellEvent;
use sigil::reactive::Runtime;
use unicode_width::UnicodeWidthStr;

use crate::keys::map_key;
use crate::mouse::map_mouse;
use crate::paint::{Grid, raster};
use crate::screen::Screen;
use sigil::profile;

pub(crate) const TICK: Duration = Duration::from_millis(33);
const BUFFER: usize = 64 * 1024;
const CELL: f32 = sigil::app::Host::CELL;

pub fn calibrate(out: &mut impl Write) -> io::Result<()> {
    set_cell_width(CellWidth::Unicode);
    let modifier = probe(out, "👍🏽")?;
    if modifier > UnicodeWidthStr::width("👍🏽") {
        let joiner = probe(out, "\u{200d}")?;
        let nonjoiner = probe(out, "\u{200c}")?;
        let keycap = probe(out, "1\u{fe0f}\u{20e3}")?.saturating_sub(1);
        set_cell_width(CellWidth::Scalar {
            joiner,
            nonjoiner,
            keycap,
        });
    }
    execute!(out, cursor::MoveTo(0, 0), Clear(ClearType::CurrentLine))
}

fn probe(out: &mut impl Write, text: &str) -> io::Result<usize> {
    execute!(
        out,
        cursor::MoveTo(0, 0),
        Clear(ClearType::CurrentLine),
        Print(text)
    )?;
    let (col, row) = cursor::position()?;
    if row != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "terminal width probe wrapped",
        ));
    }
    Ok(usize::from(col))
}

/// Called each loop turn to push externally sourced shell events.
pub type EventPump<'a> = Box<dyn FnMut(&mut Vec<ShellEvent>) + 'a>;

/// Called after each `Host::frame`, before the frame is presented.
pub type AfterFrame<'a> = Box<dyn FnMut(&mut Host, &mut EventRouter, &Screen) + 'a>;

/// Loop-level hooks for hosts embedding `run`: an external message pump and a
/// post-frame callback. `run` is `run_with` with default hooks.
#[derive(Default)]
pub struct Hooks<'a> {
    /// External message pump; see `EventPump`.
    pub external_events: Option<EventPump<'a>>,
    /// Post-frame callback; see `AfterFrame`.
    pub after_frame: Option<AfterFrame<'a>>,
}

pub fn run<C, T>(config: C, tree: T) -> AppResult<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    run_with(config, tree, &mut Hooks::default())
}

pub fn run_with<C, T>(config: C, tree: T, hooks: &mut Hooks<'_>) -> AppResult<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    let config = config.into();
    let result = Runtime::scope("sigil-tui", || live(&config, tree, hooks));
    profile::dump_if_enabled();
    result
}

pub fn present(
    tree: impl TreeRoot,
    cols: u16,
    rows: u16,
    events: &[ShellEvent],
) -> Result<String, LayoutError> {
    Runtime::scope("sigil-tui-present", || {
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(tree);
        prime(&mut host, &mut router, cols, rows)?;
        Ok(step(&mut host, &mut router, events.iter().cloned(), cols, rows)?.text())
    })
}

pub fn step(
    host: &mut Host,
    router: &mut EventRouter,
    events: impl IntoIterator<Item = ShellEvent>,
    cols: u16,
    rows: u16,
) -> Result<Grid, LayoutError> {
    let mut grid = None;
    let hooks = step_hooks(events, &mut grid, cols, rows);
    let out = host.frame(router, view(cols, rows), TICK.as_secs_f32(), hooks);
    out.layout?;
    Ok(grid.expect("paint runs after a successful layout"))
}

fn step_hooks<'e>(
    events: impl IntoIterator<Item = ShellEvent> + 'e,
    grid_slot: &'e mut Option<Grid>,
    cols: u16,
    rows: u16,
) -> FrameHooks<'e> {
    FrameHooks {
        input: Some(Box::new(move |host, router| {
            host.route_all(router, events);
        })),
        paint: Some(Box::new(move |host, _router, info| {
            if info.layout_ok {
                *grid_slot = Some(raster(&host.painted(), cols, rows, CELL));
            }
        })),
        ..FrameHooks::default()
    }
}

fn prime(
    host: &mut Host,
    router: &mut EventRouter,
    cols: u16,
    rows: u16,
) -> Result<(), LayoutError> {
    host.layout(view(cols, rows))?;
    host.repair_focus(router);
    Ok(())
}

pub(crate) fn view(cols: u16, rows: u16) -> (f32, f32) {
    (f32::from(cols) * CELL, f32::from(rows) * CELL)
}

fn prepare() -> AppResult<sigil::tasks::AsyncRuntime> {
    sigil::app::clear_exit();
    let async_rt = sigil::tasks::AsyncRuntime::new().map_err(Error::new)?;
    async_rt.install_current();
    Ok(async_rt)
}

fn live<T: TreeRoot>(config: &AppConfig, tree: T, hooks: &mut Hooks<'_>) -> AppResult<()> {
    let mut async_rt = prepare()?;
    let mut term = Term::enter().map_err(Error::new)?;
    let _hooks = HookGuard::install();
    let (mut cols, mut rows) = crossterm::terminal::size().map_err(Error::new)?;
    limit(config, &mut cols, &mut rows);
    let mut host = Host::cells();
    let mut router = EventRouter::new();
    crate::clipboard::install_clipboard();
    host.mount(tree);
    prime(&mut host, &mut router, cols, rows).map_err(Error::new)?;
    let mut screen = Screen::new(cols, rows);
    let mut pointer = sigil::hal::Point::new(0.0, 0.0);
    let mut first = true;
    let mut last = std::time::Instant::now();
    loop {
        let dt = last.elapsed().as_secs_f32().min(0.1);
        last = std::time::Instant::now();
        screen.set_dt(dt);
        let _ = async_rt.pump();
        let incoming = match gather_input(&mut first, config, &mut pointer, hooks)? {
            Incoming::Quit => break,
            Incoming::Events(events) => events,
            Incoming::Resized(next_cols, next_rows, events) => {
                cols = next_cols;
                rows = next_rows;
                screen.invalidate();
                term.redraw = true;
                events
            }
        };
        profile::begin_frame();
        let mut turn = Turn {
            screen: &mut screen,
            host: &mut host,
            router: &mut router,
            term: &mut term,
        };
        turn.frame_and_present(config.panic_policy, incoming, (cols, rows), hooks)?;
        if turn.screen.exited() {
            break;
        }
    }
    Ok(())
}

struct Turn<'a> {
    screen: &'a mut Screen,
    host: &'a mut Host,
    router: &'a mut EventRouter,
    term: &'a mut Term,
}

impl Turn<'_> {
    fn frame_and_present(
        &mut self,
        policy: PanicPolicy,
        incoming: Vec<ShellEvent>,
        size: (u16, u16),
        hooks: &mut Hooks<'_>,
    ) -> AppResult<()> {
        frame_isolated(
            policy,
            self.screen,
            self.host,
            self.router,
            incoming,
            size,
            self.term,
        )
        .map_err(Error::new)?;
        if let Some(after) = hooks.after_frame.as_mut() {
            after(self.host, self.router, self.screen);
        }
        {
            let _phase = profile::span("tui.present");
            self.term.paint(self.screen).map_err(Error::new)?;
        }
        profile::tick_frame();
        Ok(())
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "frame needs the screen, host, router, decoded input, size, and terminal"
)]
fn frame_isolated(
    policy: PanicPolicy,
    screen: &mut Screen,
    host: &mut Host,
    router: &mut EventRouter,
    incoming: Vec<ShellEvent>,
    size: (u16, u16),
    term: &mut Term,
) -> Result<(), LayoutError> {
    match policy {
        PanicPolicy::Abort => screen.frame(host, router, incoming, size).map(|_| ()),
        PanicPolicy::Isolate => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            screen.frame(host, router, incoming, size).map(|_| ())
        }))
        .unwrap_or_else(|payload| {
            let detail = sigil::tasks::panic_message(&*payload);
            eprintln!("sigil-tui: frame isolated after panic: {detail}");
            Runtime::recover_from_panic();
            restore();
            term.redraw = true;
            Ok(())
        }),
    }
}

enum Incoming {
    Quit,
    Events(Vec<ShellEvent>),
    Resized(u16, u16, Vec<ShellEvent>),
}

fn gather_input(
    first: &mut bool,
    config: &AppConfig,
    pointer: &mut sigil::hal::Point,
    hooks: &mut Hooks<'_>,
) -> AppResult<Incoming> {
    let mut incoming = Vec::new();
    let mut resize = None;
    if !core::mem::take(first) {
        match poll_input(config, pointer)? {
            Input::Quit => return Ok(Incoming::Quit),
            Input::Event(mapped) => incoming.push(mapped),
            Input::Resize(cols, rows) => resize = Some((cols, rows)),
            Input::None => {}
        }
    }
    if let Some(pump) = hooks.external_events.as_mut() {
        pump(&mut incoming);
    }
    Ok(match resize {
        Some((cols, rows)) => Incoming::Resized(cols, rows, incoming),
        None => Incoming::Events(incoming),
    })
}

enum Input {
    Quit,
    Event(ShellEvent),
    Resize(u16, u16),
    None,
}

fn poll_input(config: &AppConfig, pointer: &mut sigil::hal::Point) -> AppResult<Input> {
    if !event::poll(TICK).map_err(Error::new)? {
        return Ok(Input::None);
    }
    Ok(map_input(
        event::read().map_err(Error::new)?,
        config,
        pointer,
    ))
}

fn map_input(event: Event, config: &AppConfig, pointer: &mut sigil::hal::Point) -> Input {
    match event {
        Event::Key(key) => {
            if quit(&key, config.esc_exits) {
                Input::Quit
            } else if let Some(mapped) = map_key(&key) {
                Input::Event(mapped)
            } else {
                Input::None
            }
        }
        Event::Resize(cols, rows) => Input::Resize(cols, rows),
        Event::Mouse(mouse) => map_mouse(&mouse, CELL, pointer).map_or(Input::None, Input::Event),
        Event::Paste(pasted) => Input::Event(crate::keys::paste(&pasted)),
        Event::FocusGained | Event::FocusLost => Input::None,
    }
}

fn limit(config: &AppConfig, cols: &mut u16, rows: &mut u16) {
    if config.size.0 > 0 {
        *cols = (*cols).min(u16::try_from(config.size.0).unwrap_or(*cols));
    }
    if config.size.1 > 0 {
        *rows = (*rows).min(u16::try_from(config.size.1).unwrap_or(*rows));
    }
}

fn quit(key: &crossterm::event::KeyEvent, esc_exits: bool) -> bool {
    use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
    if key.kind == KeyEventKind::Release {
        return false;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        return true;
    }
    esc_exits && key.code == KeyCode::Esc
}

struct Term {
    redraw: bool,
    out: BufWriter<io::Stdout>,
}

impl Term {
    fn enter() -> io::Result<Self> {
        RESTORED.store(false, Ordering::Relaxed);
        let guard = EnterGuard;
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste,
            cursor::Hide
        )?;
        let _ = execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                    | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
            )
        );
        core::mem::forget(guard);
        Ok(Self {
            redraw: true,
            out: BufWriter::with_capacity(BUFFER, io::stdout()),
        })
    }

    fn paint(&mut self, screen: &mut Screen) -> io::Result<()> {
        let redraw = core::mem::take(&mut self.redraw);
        paint_frame(&mut self.out, screen, redraw)
    }
}

fn paint_frame(out: &mut impl Write, screen: &mut Screen, redraw: bool) -> io::Result<()> {
    if !redraw && !screen.needs_flush() {
        return Ok(());
    }
    queue!(out, Print("\x1b[?2026h"))?;
    if redraw {
        queue!(out, Clear(ClearType::All))?;
        screen.invalidate();
    }
    screen.flush(out)?;
    queue!(out, Print("\x1b[?2026l"))?;
    out.flush()
}

impl Drop for Term {
    fn drop(&mut self) {
        restore();
    }
}

static RESTORED: AtomicBool = AtomicBool::new(false);

fn restore() {
    if RESTORED.swap(true, Ordering::Relaxed) {
        return;
    }
    let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    let _ = execute!(
        io::stdout(),
        DisableBracketedPaste,
        DisableMouseCapture,
        cursor::Show,
        LeaveAlternateScreen
    );
    let _ = disable_raw_mode();
}

struct EnterGuard;

impl Drop for EnterGuard {
    fn drop(&mut self) {
        restore();
    }
}

type PanicHook = Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync + 'static>;

struct HookGuard {
    previous: Option<PanicHook>,
}

impl HookGuard {
    fn install() -> Self {
        let ui_thread = std::thread::current().id();
        let previous: std::sync::Arc<PanicHook> = std::sync::Arc::new(std::panic::take_hook());
        let chained = std::sync::Arc::clone(&previous);
        std::panic::set_hook(Box::new(move |info| {
            if std::thread::current().id() == ui_thread {
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
#[path = "run_tests.rs"]
mod tests;
