//! Live and headless pumps over [`sigil_app::Host`].

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
/// Frame buffer for terminal writes. A full redraw of a wide terminal is
/// tens of kilobytes; anything smaller splits one frame across syscalls.
const BUFFER: usize = 64 * 1024;
const CELL: f32 = sigil_app::Host::CELL;

/// Run a Sigil tree on the terminal. Same config and tree as
/// `sigil::launch`.
///
/// # Errors
///
/// Returns when the terminal cannot enter raw mode, or when a write fails.
pub fn run<C, T>(config: C, tree: T) -> AppResult<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    let config = config.into();
    Runtime::scope("sigil-tui", || live(&config, tree))
}

/// Headless: mount, route `events`, return the painted text.
///
/// # Errors
///
/// Returns the layout engine error instead of painting stale geometry.
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

/// Frame buffers for a terminal host.
///
/// A host that keeps one between frames reuses the paint scratch and both
/// cell buffers, so a steady frame allocates nothing. [`step`] is the
/// allocating equivalent for tests and one-shot renders.
pub struct Screen {
    nodes: Vec<Painted>,
    grid: Grid,
    prev: Grid,
    fresh: bool,
    /// The last frame differs from the one on screen.
    dirty: bool,
}

impl Screen {
    /// Buffers for a `cols` by `rows` terminal.
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

    /// Route `events`, tick, lay out, and paint one frame.
    ///
    /// # Errors
    ///
    /// Returns the layout engine error instead of painting stale geometry.
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
        // An idle app paints the same tree 30 times a second. When no record
        // moved and the terminal is the same size, the cells are the ones
        // already on screen: nothing to rasterize and nothing to send.
        let moved = host.repaint(&mut self.nodes);
        self.dirty = moved || resized || self.fresh;
        if self.dirty {
            self.grid.resize(cols, rows);
            raster_into(&mut self.grid, &self.nodes, CELL);
            return Ok(&self.grid);
        }
        // `flush` swaps the buffers, so after one the frame on screen is
        // `prev` and `grid` is the retired one. A skipped frame changed
        // nothing, so what is on screen is this frame.
        Ok(&self.prev)
    }

    /// Write what changed since the previous frame, then keep it as the new
    /// comparison point.
    ///
    /// # Errors
    ///
    /// Returns any write error from `out`.
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

    /// Repaint every cell on the next flush.
    pub const fn invalidate(&mut self) {
        self.fresh = true;
    }

    /// Whether the current frame has terminal bytes to flush.
    #[must_use]
    pub const fn needs_flush(&self) -> bool {
        self.dirty
    }
}

/// One frame: route events, tick, lay out, rasterize.
///
/// # Errors
///
/// Returns the layout engine error instead of rasterizing stale geometry.
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
    /// One buffer for a whole frame.
    ///
    /// A frame is thousands of short writes — a cursor move and a run of
    /// glyphs per row. `io::stdout` is line-buffered behind a lock, so each
    /// of those would be a syscall's worth of work and a lock acquire.
    out: BufWriter<io::Stdout>,
}

impl Term {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        // Mouse reporting: without it the terminal sends no pointer events at
        // all, and every hover, click, and wheel in the widget set is dead.
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
    // Synchronised update: the terminal shows the whole frame at once
    // rather than tearing partway through the diff.
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

/// Put the terminal back the way it was found. Safe to call twice.
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

/// Restore the terminal before a panic prints.
///
/// The default hook writes to a screen that is still in raw mode and still
/// showing the alternate buffer, so the message scrolls into a buffer that is
/// about to be discarded: the app vanishes and takes the reason with it.
/// Restoring first puts the panic where the user can read it.
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
