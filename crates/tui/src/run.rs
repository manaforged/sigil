use std::io::{self, BufWriter, Write};
use std::time::Duration;

use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event,
};
use crossterm::style::Print;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{cursor, execute, queue};
use sigil::animation_runtime::tick as tick_animations;
use sigil::app::{
    AppConfig, AppResult, CellWidth, Error, Host, LayoutError, TreeRoot, set_cell_width,
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

pub fn run<C, T>(config: C, tree: T) -> AppResult<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    let config = config.into();
    let result = Runtime::scope("sigil-tui", || live(&config, tree));
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
    Runtime::begin_effect();
    for event in events {
        host.route(router, event);
    }
    tick_animations(TICK.as_secs_f32());
    crate::timer::advance(TICK.as_secs_f32());
    crate::work::collect_work();
    Runtime::begin_layout();
    let layout = host.layout(view(cols, rows));
    if layout.is_ok() {
        host.repair_focus(router);
    }
    Runtime::begin_paint();
    let painted = layout
        .is_ok()
        .then(|| raster(&host.painted(), cols, rows, CELL));
    Runtime::end_frame();
    layout?;
    Ok(painted.expect("paint runs after a successful layout"))
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
    async_rt.install_executor();
    Ok(async_rt)
}

fn live<T: TreeRoot>(config: &AppConfig, tree: T) -> AppResult<()> {
    let mut async_rt = prepare()?;
    let mut term = Term::enter().map_err(Error::new)?;
    let _hooks = HookGuard::install();
    let (mut cols, mut rows) = crossterm::terminal::size().map_err(Error::new)?;
    limit(config, &mut cols, &mut rows);
    let mut host = Host::cells();
    let mut router = EventRouter::new();
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
        let mut incoming = Vec::new();
        if first {
            first = false;
        } else if event::poll(TICK).map_err(Error::new)? {
            match map_input(event::read().map_err(Error::new)?, config, &mut pointer) {
                Input::Quit => break,
                Input::Event(mapped) => incoming.push(mapped),
                Input::Resize(next_cols, next_rows) => {
                    cols = next_cols;
                    rows = next_rows;
                    screen.invalidate();
                    term.redraw = true;
                }
                Input::None => {}
            }
        }
        profile::begin_frame();
        screen
            .frame(&mut host, &mut router, incoming, (cols, rows))
            .map_err(Error::new)?;
        {
            let _phase = profile::span("tui.present");
            term.paint(&mut screen).map_err(Error::new)?;
        }
        profile::tick_frame();
        if sigil::app::take_exit() {
            break;
        }
    }
    Ok(())
}

enum Input {
    Quit,
    Event(ShellEvent),
    Resize(u16, u16),
    None,
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
    use crossterm::event::{KeyCode, KeyModifiers};
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c' | 'C'))
    {
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
        enable_raw_mode()?;
        let raw = RawModeGuard;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste,
            cursor::Hide
        )?;
        core::mem::forget(raw);
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

fn restore() {
    let _ = execute!(
        io::stdout(),
        DisableBracketedPaste,
        DisableMouseCapture,
        cursor::Show,
        LeaveAlternateScreen
    );
    let _ = disable_raw_mode();
}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
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
