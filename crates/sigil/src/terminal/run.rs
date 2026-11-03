use crate::reactive::Owner;
use std::time::{Duration, Instant};

use crate::app::PanicPolicy;
use crate::events::Event;
use crate::kernel::IntoNode;
use crate::reactive::Runtime;
use crate::{AppConfig, AppResult, Error, Host};

use crate::profile;
use crate::terminal::caps::Caps;
use crate::terminal::input::{Incoming, InputWake, gather_input, limit};
use crate::terminal::paint::Grid;
use crate::terminal::screen::Screen;
use crate::terminal::signals::SignalGuard;
pub use crate::terminal::term::calibrate;
#[cfg(test)]
use crate::terminal::term::paint_frame;
use crate::terminal::term::{HookGuard, Term, restore};

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

pub(crate) fn run<N: IntoNode>(config: &AppConfig, tree: impl Fn() -> N + 'static) -> AppResult {
    let hooks = &mut config.terminal_hooks.take();
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
        Screen::new(cols, rows)
            .frame(self, events, (cols, rows))
            .cloned()
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
    #[cfg(feature = "scene")]
    if crate::terminal::pixels::wanted(config, term.caps()) {
        return crate::terminal::pixels::live(
            config,
            tree,
            (&mut async_rt, &waker),
            &mut term,
            (cols, rows),
            hooks,
        );
    }
    let mut host = Host::new(crate::kernel::HostMode::Cell);
    crate::terminal::clipboard::install_clipboard(term.caps());
    host.set_hyperlinks(term.caps().hyperlinks);
    host.mount_app(tree);
    prime(&mut host, cols, rows)?;
    let mut screen = open_screen(config, (cols, rows), term.caps());
    let mut pointer = crate::geometry::Point::new(0.0, 0.0);
    let (mut first, mut settle, mut last) = (true, true, Instant::now());
    loop {
        if crate::terminal::signals::stopped() {
            break;
        }
        let woke = pump(&host, &mut async_rt);
        let timing = turn_wait(config.frame_interval, woke, last, screen.deadline());
        let pending = inspect_pending() || due(screen.deadline());
        let input = match gather_input(&mut first, config, &mut pointer, hooks, (&waker, timing))? {
            Incoming::Quit => break,
            Incoming::Idle if idle(settle, woke, &term, pending) => continue,
            Incoming::Idle => Vec::new(),
            Incoming::Events(events) => events,
            Incoming::Resized(next_cols, next_rows, events) => {
                (cols, rows) = (next_cols, next_rows);
                term.mark_cell_stale();
                screen.invalidate();
                term.redraw = true;
                events
            }
        };
        remeasure(&mut term, &mut screen);
        if !timing.1 {
            pump(&host, &mut async_rt);
        }
        settle = !input.is_empty();
        let mut turn = Turn {
            last: &mut last,
            screen: &mut screen,
            host: &mut host,
            term: &mut term,
        };
        turn.take(config, input, (cols, rows), hooks)?;
        if turn.screen.exited() {
            break;
        }
    }
    Ok(())
}

fn remeasure(term: &mut Term, screen: &mut Screen) {
    if let Some(caps) = term.remeasure() {
        screen.set_caps(caps);
        screen.invalidate();
        term.redraw = true;
    }
}

fn open_screen(config: &AppConfig, (cols, rows): (u16, u16), caps: Caps) -> Screen {
    let mut screen = Screen::with_caps(cols, rows, caps);
    screen.set_access(config.terminal_access);
    screen.speak_above(config.inline);
    screen
}

pub(super) fn pump(host: &Host, async_rt: &mut crate::tasks::AsyncRuntime) -> usize {
    let _env = host.enter();
    async_rt.pump()
}

fn idle(settle: bool, woke: usize, term: &Term, pending: bool) -> bool {
    !settle && woke == 0 && !frame_due() && !term.pending() && !pending
}

fn due(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|due| due <= Instant::now())
}

pub(super) fn frame_due() -> bool {
    Runtime::frame_dirty()
        || crate::animation_runtime::active_count() > 0
        || crate::animation_runtime::running_clocks() > 0
        || crate::reactive::next_interval_deadline().is_some_and(|due| due <= Instant::now())
}

pub(super) fn turn_wait(
    tick: Duration,
    woke: usize,
    last: Instant,
    status: Option<Instant>,
) -> (Option<Duration>, bool) {
    if woke > 0 || frame_due() {
        return (Some(tick.saturating_sub(last.elapsed())), false);
    }
    let next = match (crate::reactive::next_interval_deadline(), status) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    let wait = next.map(|due| due.saturating_duration_since(Instant::now()).min(tick));
    (wait, true)
}

struct Turn<'a> {
    last: &'a mut Instant,
    screen: &'a mut Screen,
    host: &'a mut Host,
    term: &'a mut Term,
}

impl Turn<'_> {
    fn take(
        &mut self,
        config: &AppConfig,
        incoming: Vec<Event>,
        (cols, rows): (u16, u16),
        hooks: &mut Hooks<'_>,
    ) -> AppResult<()> {
        let size = match hooks.region_rows.as_mut() {
            Some(hook) if config.inline => (cols, hook().clamp(1, rows.max(1))),
            _ => (cols, rows),
        };
        self.screen.set_dt(self.last.elapsed().as_secs_f32());
        *self.last = Instant::now();
        profile::begin_frame(profile::BackendStatus::TERMINAL);
        let recovered = frame_isolated(config.panic_policy, self.screen, self.host, incoming, size)
            .map_err(Error::io)?;
        if recovered {
            restore();
            self.term.reenter().map_err(Error::io)?;
        }
        if let Some(after) = hooks.after_frame.as_mut() {
            after(self.host, self.screen);
        }
        for message in self.screen.take_above() {
            if let Err(err) = crate::terminal::print_above(&message) {
                tracing::warn!(target: "sigil::terminal", error = %err, "announcement write failed");
            }
        }
        {
            let _phase = profile::span(crate::profile::Phase::TuiPresent);
            self.term.paint(self.screen).map_err(Error::io)?;
        }
        profile::tick_frame();
        Ok(())
    }
}

fn frame_isolated(
    policy: PanicPolicy,
    screen: &mut Screen,
    host: &mut Host,
    incoming: Vec<Event>,
    size: (u16, u16),
) -> Result<bool, Error> {
    match policy {
        PanicPolicy::Abort => screen.frame(host, incoming, size).map(|_| false),
        PanicPolicy::Isolate => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            screen.frame(host, incoming, size).map(|_| false)
        }))
        .unwrap_or_else(|payload| {
            let detail = crate::tasks::panic_message(&*payload);
            eprintln!("sigil-terminal: frame isolated after panic: {detail}");
            Runtime::recover_from_panic();
            Ok(true)
        }),
    }
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;

fn enter(config: &AppConfig) -> AppResult<Term> {
    if let Some(tracking) = config.mouse_tracking {
        crate::terminal::set_mouse_tracking(tracking).map_err(Error::io)?;
    }
    let term = Term::enter(config).map_err(Error::io)?;
    crate::app::set_ambiguous(term.caps().ambiguous);
    let dark = term.caps().backdrop == Some(crate::terminal::Backdrop::Dark);
    if config.theme_file.is_none() && dark {
        crate::app::set_theme(crate::style::Theme::dark());
    }
    Ok(term)
}
