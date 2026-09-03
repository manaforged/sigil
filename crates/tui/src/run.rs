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
use sigil_animation_runtime::tick as tick_animations;
use sigil_app::{AppConfig, AppResult, Error, Host, LayoutError, Painted, TreeRoot};
use sigil_events::EventRouter;
use sigil_hal::ShellEvent;
use sigil_reactive::Runtime;

use crate::keys::map_key;
use crate::mouse::map_mouse;
use crate::paint::{Grid, raster, raster_into};

const TICK: Duration = Duration::from_millis(33);
const BUFFER: usize = 64 * 1024;
const CELL: f32 = sigil_app::Host::CELL;

pub fn run<C, T>(config: C, tree: T) -> AppResult<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    let config = config.into();
    Runtime::scope("sigil-tui", || live(&config, tree))
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

pub struct Screen {
    nodes: Vec<Painted>,
    grid: Grid,
    prev: Grid,
    fresh: bool,
    dirty: bool,
}

impl Screen {
    #[must_use]
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            nodes: Vec::new(),
            grid: Grid::new(cols, rows),
            prev: Grid::new(cols, rows),
            fresh: true,
            dirty: true,
        }
    }

    pub fn frame(
        &mut self,
        host: &mut Host,
        router: &mut EventRouter,
        events: impl IntoIterator<Item = ShellEvent>,
        size: (u16, u16),
    ) -> Result<&Grid, LayoutError> {
        let (cols, rows) = size;
        for event in events {
            host.route(router, event);
        }
        tick_animations(TICK.as_secs_f32());
        crate::timer::advance(TICK.as_secs_f32());
        crate::work::collect_work();
        host.layout(view(cols, rows))?;
        host.repair_focus(router);
        let resized = self.grid.cols() != cols as usize || self.grid.rows() != rows as usize;
        let moved = host.repaint(&mut self.nodes);
        self.dirty = moved || resized || self.fresh;
        if self.dirty {
            self.grid.resize(cols, rows);
            raster_into(&mut self.grid, &self.nodes, CELL);
            return Ok(&self.grid);
        }
        Ok(&self.prev)
    }

    pub fn flush(&mut self, out: &mut impl Write) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        self.dirty = false;
        if self.fresh
            || self.prev.cols() != self.grid.cols()
            || self.prev.rows() != self.grid.rows()
        {
            self.fresh = false;
            self.prev.mirror(&self.grid);
            crate::write::all(out, &self.grid)?;
        } else {
            crate::write::dirty(out, &self.prev, &self.grid)?;
        }
        core::mem::swap(&mut self.prev, &mut self.grid);
        Ok(())
    }

    pub const fn invalidate(&mut self) {
        self.fresh = true;
    }

    #[must_use]
    pub const fn needs_flush(&self) -> bool {
        self.dirty
    }
}

pub fn step(
    host: &mut Host,
    router: &mut EventRouter,
    events: impl IntoIterator<Item = ShellEvent>,
    cols: u16,
    rows: u16,
) -> Result<Grid, LayoutError> {
    for event in events {
        host.route(router, event);
    }
    tick_animations(TICK.as_secs_f32());
    crate::timer::advance(TICK.as_secs_f32());
    crate::work::collect_work();
    host.layout(view(cols, rows))?;
    host.repair_focus(router);
    Ok(raster(&host.painted(), cols, rows, CELL))
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

fn view(cols: u16, rows: u16) -> (f32, f32) {
    (f32::from(cols) * CELL, f32::from(rows) * CELL)
}

fn live<T: TreeRoot>(config: &AppConfig, tree: T) -> AppResult<()> {
    guard();
    sigil_app::clear_exit();
    let mut term = Term::enter().map_err(Error::new)?;
    let (mut cols, mut rows) = crossterm::terminal::size().map_err(Error::new)?;
    limit(config, &mut cols, &mut rows);
    let mut host = Host::cells();
    let mut router = EventRouter::new();
    host.mount(tree);
    prime(&mut host, &mut router, cols, rows).map_err(Error::new)?;
    let mut screen = Screen::new(cols, rows);
    let mut pointer = sigil_hal::Point::new(0.0, 0.0);
    let mut first = true;
    loop {
        let mut incoming = Vec::new();
        if first {
            first = false;
        } else if event::poll(TICK).map_err(Error::new)? {
            match event::read().map_err(Error::new)? {
                Event::Key(key) => {
                    if quit(&key, config.esc_exits) {
                        break;
                    }
                    if let Some(mapped) = map_key(&key) {
                        incoming.push(mapped);
                    }
                }
                Event::Resize(next_cols, next_rows) => {
                    cols = next_cols;
                    rows = next_rows;
                    screen.invalidate();
                    term.redraw = true;
                }
                Event::Mouse(mouse) => {
                    if let Some(mapped) = map_mouse(&mouse, CELL, &mut pointer) {
                        incoming.push(mapped);
                    }
                }
                Event::Paste(pasted) => incoming.push(crate::keys::paste(&pasted)),
                Event::FocusGained | Event::FocusLost => {}
            }
        }
        screen
            .frame(&mut host, &mut router, incoming, (cols, rows))
            .map_err(Error::new)?;
        term.paint(&mut screen).map_err(Error::new)?;
        if sigil_app::take_exit() {
            break;
        }
    }
    Ok(())
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
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste,
            cursor::Hide
        )?;
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

fn guard() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
    });
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
