//! Live and headless pumps over [`sigil_app::Host`].

use std::io::{self, Write};
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{cursor, execute, queue};
use sigil_animation_runtime::tick as tick_animations;
use sigil_app::{AppConfig, AppResult, Error, Host, TreeRoot};
use sigil_events::EventRouter;
use sigil_hal::ShellEvent;
use sigil_reactive::Runtime;

use crate::keys::map_key;
use crate::paint::{Grid, raster};

const TICK: Duration = Duration::from_millis(33);
const CELL: f32 = sigil_app::Host::CELL;

/// Run a Sigil tree on the terminal. Same config and tree as `sigil::launch`.
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
#[must_use]
pub fn present(tree: impl TreeRoot, cols: u16, rows: u16, events: &[ShellEvent]) -> String {
    Runtime::scope("sigil-tui-present", || {
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(tree);
        prime(&mut host, &mut router, cols, rows);
        step(&mut host, &mut router, events.iter().cloned(), cols, rows).text()
    })
}

/// One frame: route events, tick, lay out, rasterize.
pub fn step(
    host: &mut Host,
    router: &mut EventRouter,
    events: impl IntoIterator<Item = ShellEvent>,
    cols: u16,
    rows: u16,
) -> Grid {
    for event in events {
        host.route(router, event);
    }
    tick_animations(TICK.as_secs_f32());
    let _ = host.layout(view(cols, rows));
    raster(&host.painted(), cols, rows, CELL)
}

fn prime(host: &mut Host, router: &mut EventRouter, cols: u16, rows: u16) {
    let _ = host.layout(view(cols, rows));
    router.focus_next(host.cx());
}

fn view(cols: u16, rows: u16) -> (f32, f32) {
    (f32::from(cols) * CELL, f32::from(rows) * CELL)
}

fn live<T: TreeRoot>(config: &AppConfig, tree: T) -> AppResult<()> {
    let mut term = Term::enter().map_err(Error::new)?;
    let (mut cols, mut rows) = crossterm::terminal::size().map_err(Error::new)?;
    if config.size.0 > 0 {
        cols = cols.min(u16::try_from(config.size.0).unwrap_or(cols));
    }
    if config.size.1 > 0 {
        rows = rows.min(u16::try_from(config.size.1).unwrap_or(rows));
    }
    let mut host = Host::cells();
    let mut router = EventRouter::new();
    host.mount(tree);
    prime(&mut host, &mut router, cols, rows);
    let mut prev = Grid::new(cols, rows);
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
                    prev = Grid::new(cols, rows);
                    term.redraw = true;
                }
                Event::FocusGained | Event::FocusLost | Event::Mouse(_) | Event::Paste(_) => {}
            }
        }
        let grid = step(&mut host, &mut router, incoming, cols, rows);
        term.paint(&prev, &grid).map_err(Error::new)?;
        prev = grid;
    }
    Ok(())
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
}

impl Term {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
        Ok(Self { redraw: true })
    }

    fn paint(&mut self, prev: &Grid, next: &Grid) -> io::Result<()> {
        let mut out = io::stdout();
        queue!(out, crossterm::style::Print("\x1b[?2026h"))?;
        if self.redraw || prev.cols() != next.cols() || prev.rows() != next.rows() {
            queue!(
                out,
                crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
            )?;
            self.redraw = false;
            write_all(&mut out, next)?;
        } else {
            write_dirty(&mut out, prev, next)?;
        }
        queue!(out, crossterm::style::Print("\x1b[?2026l"))?;
        out.flush()
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), cursor::Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

fn write_all(out: &mut impl Write, grid: &Grid) -> io::Result<()> {
    for row in 0..grid.rows() {
        write_row(out, grid, row)?;
    }
    Ok(())
}

fn write_dirty(out: &mut impl Write, prev: &Grid, next: &Grid) -> io::Result<()> {
    for row in 0..next.rows() {
        if row_changed(prev, next, row) {
            write_row(out, next, row)?;
        }
    }
    Ok(())
}

fn row_changed(prev: &Grid, next: &Grid, row: usize) -> bool {
    for col in 0..next.cols() {
        if prev.get(col, row) != next.get(col, row) {
            return true;
        }
    }
    false
}

fn write_row(out: &mut impl Write, grid: &Grid, row: usize) -> io::Result<()> {
    let y = u16::try_from(row.saturating_add(1)).unwrap_or(1);
    queue!(out, cursor::MoveTo(0, y.saturating_sub(1)))?;
    let mut line = String::new();
    for col in 0..grid.cols() {
        let ch = grid.get(col, row).map_or(' ', |cell| cell.ch);
        line.push(ch);
    }
    queue!(out, crossterm::style::Print(line))
}
