use crate::reactive::Owner;
use std::time::{Duration, Instant};

use crate::events::Event;
use crate::kernel::IntoNode;
use crate::{AppConfig, AppResult, Error, Host};

use crate::profile;
use crate::terminal::caps::Capabilities;
use crate::terminal::input::{Incoming, InputWake, gather_input, limit};
use crate::terminal::paint::Grid;
use crate::terminal::pixels::Pixels;
use crate::terminal::screen::Screen;
use crate::terminal::signals::SignalGuard;
pub(crate) use crate::terminal::term::calibrate;
#[cfg(test)]
use crate::terminal::term::paint_frame;
use crate::terminal::term::{HookGuard, Term};

mod turn;

use turn::Turn;
pub(super) use turn::{isolated, recover};

pub(crate) const CELL: f32 = crate::Host::CELL;

/// Reads the next batch of events for an embedded terminal loop.
pub(crate) type EventPump<'a> = Box<dyn FnMut(&mut Vec<Event>) + 'a>;

/// Runs after each terminal frame.
pub(crate) type AfterFrame<'a> = Box<dyn FnMut(&mut Host, &Screen) + 'a>;

/// The hooks of an embedded terminal loop.
#[derive(Default)]
pub struct Hooks<'a> {
    /// The external events, if any.
    pub external_events: Option<EventPump<'a>>,
    /// The after frame, if any.
    pub after_frame: Option<AfterFrame<'a>>,
    /// The region rows, if any.
    pub region_rows: Option<Box<dyn FnMut() -> u16 + 'a>>,
}

impl core::fmt::Debug for Hooks<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Hooks").finish_non_exhaustive()
    }
}

pub(crate) fn run<N: IntoNode>(config: &AppConfig, tree: impl Fn() -> N + 'static) -> AppResult {
    let hooks = &mut config.terminal.hooks.take();
    #[cfg(feature = "inspect")]
    let inspect = crate::app::inspect::install();
    #[cfg(feature = "inspect")]
    if let Some(inspect) = &inspect {
        let mut pump = hooks.external_events.take();
        let drain = inspect.clone();
        hooks.external_events = Some(Box::new(move |events| {
            drain.drain_events(events, CELL);
            if let Some(pump) = pump.as_mut() {
                pump(events);
            }
        }));
        let mut after = hooks.after_frame.take();
        let svc = inspect.clone();
        hooks.after_frame = Some(Box::new(move |host, screen| {
            svc.service(
                host,
                crate::app::inspect::InspectView {
                    cell: Some(CELL),
                    text: Some(&screen.text()),
                    shot: None,
                },
            );
            if let Some(hook) = after.as_mut() {
                hook(host, screen);
            }
        }));
    }
    #[cfg(feature = "inspect")]
    let inspect_pending = move || {
        inspect
            .as_ref()
            .is_some_and(crate::app::inspect::Inspect::has_pending)
    };
    #[cfg(not(feature = "inspect"))]
    let inspect_pending = || false;
    let result = Owner::scope("sigil-terminal", || {
        live(config, tree, hooks, inspect_pending)
    });
    if let Some(hook) = &config.exit.0 {
        hook();
    }
    profile::dump_if_enabled();
    result
}

#[cfg(test)]
pub(crate) fn present<N: IntoNode>(
    tree: impl Fn() -> N + 'static,
    cols: u16,
    rows: u16,
    events: &[Event],
) -> Result<String, Error> {
    Owner::scope("sigil-terminal-present", || {
        let mut host = Host::new(crate::kernel::HostMode::Cell);
        host.mount(tree);
        prime(&mut host, cols, rows)?;
        Ok(host.grid(events.iter().cloned(), cols, rows)?.text())
    })
}

#[cfg(test)]
pub(crate) fn step(
    host: &mut Host,
    events: impl IntoIterator<Item = Event>,
    cols: u16,
    rows: u16,
) -> Result<Grid, Error> {
    host.grid(events, cols, rows)
}

impl Host {
    /// Runs one terminal frame and returns its cell grid.
    pub fn grid(
        &mut self,
        events: impl IntoIterator<Item = Event>,
        cols: u16,
        rows: u16,
    ) -> Result<Grid, Error> {
        let mut screen = self.take_cell_screen().unwrap_or_else(|| {
            self.reset_emit();
            Box::new(Screen::new(cols, rows))
        });
        let out = screen.grid_frame(self, events, (cols, rows));
        self.put_cell_screen(screen);
        out
    }

    /// Writes the ANSI bytes that turn the `prev` grid into the `next` grid.
    pub fn ansi(
        out: &mut impl std::io::Write,
        prev: &Grid,
        next: &Grid,
        depth: crate::terminal::ColorDepth,
    ) -> Result<(), Error> {
        crate::terminal::write::render(out, prev, next, depth)
    }

    /// Draws a scene into a cell grid with this host's theme.
    #[doc(hidden)]
    #[cfg(feature = "kittest")]
    pub fn paint_cells(&self, grid: &mut Grid, scene: &crate::scene::CommandScene) {
        crate::terminal::paint::raster_into(grid, scene, self.theme(), CELL);
    }
}

fn prime(host: &mut Host, cols: u16, rows: u16) -> Result<(), Error> {
    host.layout(view(cols, rows))?;
    host.repair_focus();
    Ok(())
}

pub(crate) fn view(cols: u16, rows: u16) -> (f32, f32) {
    (f32::from(cols) * CELL, f32::from(rows) * CELL)
}

type Prepared = (
    crate::tasks::AsyncRuntime,
    InputWake,
    Option<crate::app::ThemeWatcher>,
);

type Setup = (Prepared, Term, (SignalGuard, HookGuard), (u16, u16));

fn setup(config: &AppConfig) -> AppResult<Setup> {
    crate::app::clear_exit();
    let mut async_rt = crate::tasks::AsyncRuntime::with_workers(config.task_workers);
    let wake = InputWake::install(&mut async_rt);
    async_rt.install_current();
    let theme = crate::app::launch_theme(config.theme_file.as_deref());
    crate::terminal::signals::resume();
    let hooks = HookGuard::install();
    let signals = SignalGuard::install().map_err(Error::io)?;
    let term = enter(config)?;
    let (mut cols, mut rows) = crossterm::terminal::size().map_err(Error::io)?;
    limit(config, &mut cols, &mut rows);
    Ok((
        (async_rt, wake, theme),
        term,
        (signals, hooks),
        (cols, rows),
    ))
}

fn live<N: IntoNode>(
    config: &AppConfig,
    tree: impl Fn() -> N + 'static,
    hooks: &mut Hooks<'_>,
    inspect_pending: impl Fn() -> bool,
) -> AppResult {
    let ((mut async_rt, waker, _theme_watch), mut term, _guards, (mut cols, mut rows)) =
        setup(config)?;
    let tree = std::rc::Rc::new(tree);
    let _follow = crate::terminal::backdrop::Follow::start(term.caps().backdrop);
    let mut host = mount_cell(&tree, &term, (cols, rows))?;
    host.set_quit(config.quit);
    let mut screen = open_screen(config, (cols, rows), term.caps());
    let mut pixels = Pixels::open(config, term.caps());
    let mut pointer = crate::geometry::Point::new(0.0, 0.0);
    let (mut first, mut settle, mut last) = (true, true, Instant::now());
    while !crate::terminal::signals::stopped() {
        let woke = pump(pixels.host().unwrap_or(&host), &mut async_rt);
        settle |= follow_backdrop(config, &mut term, &mut screen);
        if pixels.sync(&tree, &mut host).map_err(Error::io)? {
            settle = true;
            invalidate(&mut term, &mut screen);
        }
        let deadline = pixels.host().map_or_else(|| screen.deadline(), |_| None);
        let timing = turn_wait(config.frame_interval, woke, last, deadline);
        let pending = inspect_pending() || due(deadline);
        let input = match gather_input(&mut first, config, &mut pointer, hooks, (&waker, timing))? {
            Incoming::Idle if idle(settle, woke, &term, pending) => continue,
            Incoming::Idle => Vec::new(),
            Incoming::Events(events) => events,
            Incoming::Resized(next_cols, next_rows, events) => {
                (cols, rows) = (next_cols, next_rows);
                term.mark_cell_stale();
                invalidate(&mut term, &mut screen);
                events
            }
        };
        remeasure(&mut term, &mut screen);
        settle = !input.is_empty();
        if pixels.host().is_some() {
            let dt = tick(&mut last);
            if pixels.turn(config.panic_policy, (input, dt), (cols, rows), &mut term)? {
                break;
            }
            continue;
        }
        if !timing.1 {
            pump(&host, &mut async_rt);
        }
        let mut turn = Turn::new(&mut last, &mut screen, &mut host, &mut term);
        if turn.take(config, input, (cols, rows), hooks)? {
            break;
        }
    }
    pixels.close(term.caps().mux).map_err(Error::io)
}

fn mount_cell<N: IntoNode>(
    tree: &std::rc::Rc<impl Fn() -> N + 'static>,
    term: &Term,
    (cols, rows): (u16, u16),
) -> AppResult<Host> {
    let mut host = Host::new(crate::kernel::HostMode::Cell);
    crate::terminal::clipboard::install_clipboard(term.caps());
    host.hyperlinks(term.caps().hyperlinks);
    let shared = tree.clone();
    host.mount_app(move || shared());
    prime(&mut host, cols, rows)?;
    Ok(host)
}

fn follow_backdrop(config: &AppConfig, term: &mut Term, screen: &mut Screen) -> bool {
    let Some(backdrop) = crate::terminal::backdrop()
        .peek()
        .filter(|now| term.caps().backdrop != Some(*now))
    else {
        return false;
    };
    term.set_backdrop(backdrop);
    default_theme(config, backdrop);
    screen.invalidate();
    true
}

const fn invalidate(term: &mut Term, screen: &mut Screen) {
    screen.invalidate();
    term.redraw = true;
}

fn tick(last: &mut Instant) -> f32 {
    let dt = last.elapsed().as_secs_f32();
    *last = Instant::now();
    dt
}

fn remeasure(term: &mut Term, screen: &mut Screen) {
    if let Some(caps) = term.remeasure() {
        screen.set_caps(caps);
        invalidate(term, screen);
    }
}

fn open_screen(config: &AppConfig, (cols, rows): (u16, u16), caps: Capabilities) -> Screen {
    let mut screen = Screen::new(cols, rows);
    screen.set_caps(caps);
    screen.access(config.terminal.access);
    screen.speak_above(config.inline);
    screen
}

pub(super) fn pump(host: &Host, async_rt: &mut crate::tasks::AsyncRuntime) -> usize {
    let _env = host.enter();
    async_rt.pump()
}

fn idle(settle: bool, woke: usize, term: &Term, pending: bool) -> bool {
    !settle && woke == 0 && !Host::frame_due() && !term.pending() && !pending
}

fn due(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|due| due <= Instant::now())
}

pub(super) fn turn_wait(
    tick: Duration,
    woke: usize,
    last: Instant,
    status: Option<Instant>,
) -> (Option<Duration>, bool) {
    if woke > 0 || Host::frame_due() {
        return (Some(tick.saturating_sub(last.elapsed())), false);
    }
    let next = match (crate::reactive::next_interval_deadline(), status) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    let wait = next.map(|due| due.saturating_duration_since(Instant::now()).min(tick));
    (wait, true)
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;

fn enter(config: &AppConfig) -> AppResult<Term> {
    if let Some(tracking) = config.terminal.mouse_tracking {
        crate::terminal::mouse_tracking(tracking).map_err(Error::io)?;
    }
    let term = Term::enter(config).map_err(Error::io)?;
    crate::app::ambiguous(term.caps().ambiguous);
    if term.caps().backdrop == Some(crate::terminal::Backdrop::Dark) {
        default_theme(config, crate::terminal::Backdrop::Dark);
    }
    Ok(term)
}

fn default_theme(config: &AppConfig, backdrop: crate::terminal::Backdrop) {
    if config.theme_file.is_none() {
        (match backdrop {
            crate::terminal::Backdrop::Dark => crate::style::Theme::dark(),
            crate::terminal::Backdrop::Light => crate::style::Theme::light(),
        })
        .apply();
    }
}
