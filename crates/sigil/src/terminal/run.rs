use std::io::{self, Write};
use std::time::Duration;

use crate::app::{CellWidth, PanicPolicy, set_cell_width};
use crate::events::EventRouter;
use crate::events::ShellEvent;
use crate::kernel::IntoNode;
use crate::layout::LayoutError;
use crate::reactive::Runtime;
use crate::{AppConfig, AppResult, Error, Host};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, execute};
use unicode_width::UnicodeWidthStr;

use crate::profile;
use crate::terminal::caps::Caps;
use crate::terminal::input::{Incoming, Wake, gather_input, limit};
use crate::terminal::paint::Grid;
use crate::terminal::screen::Screen;
use crate::terminal::signals::SignalGuard;
#[cfg(test)]
use crate::terminal::term::paint_frame;
use crate::terminal::term::{HookGuard, Term, restore};

pub(crate) const CELL: f32 = crate::Host::CELL;

pub fn calibrate(out: &mut impl Write) -> io::Result<()> {
    set_cell_width(CellWidth::Unicode);
    let home = cursor::position()?;
    let modifier = probe(out, home, "👍🏽")?;
    if modifier > UnicodeWidthStr::width("👍🏽") {
        let joiner = probe(out, home, "\u{200d}")?;
        let nonjoiner = probe(out, home, "\u{200c}")?;
        let keycap = probe(out, home, "1\u{fe0f}\u{20e3}")?.saturating_sub(1);
        set_cell_width(CellWidth::Scalar {
            joiner,
            nonjoiner,
            keycap,
        });
    }
    execute!(
        out,
        cursor::MoveTo(0, home.1),
        Clear(ClearType::CurrentLine)
    )
}

fn probe(out: &mut impl Write, home: (u16, u16), text: &str) -> io::Result<usize> {
    execute!(
        out,
        cursor::MoveTo(0, home.1),
        Clear(ClearType::CurrentLine),
        Print(text)
    )?;
    let (col, row) = cursor::position()?;
    if row != home.1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "terminal width probe wrapped",
        ));
    }
    Ok(usize::from(col))
}

pub type EventPump<'a> = Box<dyn FnMut(&mut Vec<ShellEvent>) + 'a>;

pub type AfterFrame<'a> = Box<dyn FnMut(&mut Host, &mut EventRouter, &Screen) + 'a>;

#[derive(Default)]
pub struct Hooks<'a> {
    pub external_events: Option<EventPump<'a>>,
    pub after_frame: Option<AfterFrame<'a>>,
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
        hooks.after_frame = Some(Box::new(move |host, router, screen| {
            svc.service(
                host,
                router,
                crate::app::inspect::View {
                    cell: Some(CELL),
                    text: Some(&screen.text()),
                    shot: None,
                },
            );
            if let Some(hook) = after.as_mut() {
                hook(host, router, screen);
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
    let result = Runtime::scope("sigil-terminal", || {
        live(config, tree, hooks, inspect_pending)
    });
    profile::dump_if_enabled();
    result
}

pub fn present<N: IntoNode>(
    tree: impl Fn() -> N + 'static,
    cols: u16,
    rows: u16,
    events: &[ShellEvent],
) -> Result<String, LayoutError> {
    Runtime::scope("sigil-terminal-present", || {
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
    let mut screen = Screen::new(cols, rows);
    screen.frame(host, router, events, (cols, rows)).cloned()
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

type Prepared = (
    crate::tasks::AsyncRuntime,
    Wake,
    Option<crate::app::ThemeWatcher>,
);

fn prepare(config: &AppConfig) -> AppResult<Prepared> {
    crate::app::clear_exit();
    let mut async_rt =
        crate::tasks::AsyncRuntime::with_workers(config.task_workers).map_err(Error::task)?;
    let wake = Wake::install(&mut async_rt);
    async_rt.install_current();
    Ok((
        async_rt,
        wake,
        crate::app::launch_theme(config.theme_file.as_deref()),
    ))
}

type Setup = (Prepared, Term, (SignalGuard, HookGuard), (u16, u16));

fn setup(config: &AppConfig) -> AppResult<Setup> {
    let prepared = prepare(config)?;
    let term = enter(config)?;
    let hooks = HookGuard::install();
    let signals = SignalGuard::install().map_err(Error::io)?;
    Ok((prepared, term, (signals, hooks), term_size(config)?))
}

fn live<N: IntoNode>(
    config: &AppConfig,
    tree: impl Fn() -> N + 'static,
    hooks: &mut Hooks<'_>,
    inspect_pending: impl Fn() -> bool,
) -> AppResult {
    let ((mut async_rt, waker, _theme_watch), mut term, _guards, (mut cols, mut rows)) =
        setup(config)?;
    let mut host = Host::cells();
    let mut router = EventRouter::new();
    crate::terminal::clipboard::install_clipboard(term.caps());
    host.set_caps(term.caps().host());
    host.mount_app(tree);
    prime(&mut host, &mut router, cols, rows).map_err(Error::from)?;
    let mut screen = open_screen(config, (cols, rows), term.caps());
    let mut pointer = crate::geometry::Point::new(0.0, 0.0);
    let (mut first, mut settle, mut last) = (true, true, std::time::Instant::now());
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
                screen.invalidate();
                term.redraw = true;
                events
            }
        };
        if !timing.1 {
            pump(&host, &mut async_rt);
        }
        settle = !input.is_empty();
        let mut turn = Turn {
            last: &mut last,
            screen: &mut screen,
            host: &mut host,
            router: &mut router,
            term: &mut term,
        };
        turn.take(config, input, (cols, rows), hooks)?;
        if turn.screen.exited() {
            break;
        }
    }
    Ok(())
}

fn open_screen(config: &AppConfig, (cols, rows): (u16, u16), caps: Caps) -> Screen {
    let mut screen = Screen::with_caps(cols, rows, caps);
    screen.set_access(config.terminal_access);
    screen.speak_above(config.inline);
    screen
}

fn term_size(config: &AppConfig) -> AppResult<(u16, u16)> {
    let (mut cols, mut rows) = crossterm::terminal::size().map_err(Error::io)?;
    limit(config, &mut cols, &mut rows);
    Ok((cols, rows))
}

fn pump(host: &Host, async_rt: &mut crate::tasks::AsyncRuntime) -> usize {
    let _env = host.enter();
    async_rt.pump()
}

fn idle(settle: bool, woke: usize, term: &Term, inspect_pending: bool) -> bool {
    !settle && woke == 0 && !frame_due() && !term.pending() && !inspect_pending
}

fn due(deadline: Option<std::time::Instant>) -> bool {
    deadline.is_some_and(|due| due <= std::time::Instant::now())
}

fn frame_due() -> bool {
    Runtime::frame_dirty()
        || crate::animation_runtime::active_count() > 0
        || crate::reactive::next_interval_deadline()
            .is_some_and(|due| due <= std::time::Instant::now())
}

fn turn_wait(
    tick: Duration,
    woke: usize,
    last: std::time::Instant,
    status: Option<std::time::Instant>,
) -> (Option<Duration>, bool) {
    if woke > 0 || frame_due() {
        return (Some(tick.saturating_sub(last.elapsed())), false);
    }
    let next = match (crate::reactive::next_interval_deadline(), status) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    let wait = next.map(|due| {
        due.saturating_duration_since(std::time::Instant::now())
            .min(tick)
    });
    (wait, true)
}

struct Turn<'a> {
    last: &'a mut std::time::Instant,
    screen: &'a mut Screen,
    host: &'a mut Host,
    router: &'a mut EventRouter,
    term: &'a mut Term,
}

impl Turn<'_> {
    fn take(
        &mut self,
        config: &AppConfig,
        incoming: Vec<ShellEvent>,
        (cols, rows): (u16, u16),
        hooks: &mut Hooks<'_>,
    ) -> AppResult<()> {
        let size = match hooks.region_rows.as_mut() {
            Some(hook) if config.inline => (cols, hook().clamp(1, rows.max(1))),
            _ => (cols, rows),
        };
        self.frame_and_present(config.panic_policy, incoming, size, hooks)
    }

    fn frame_and_present(
        &mut self,
        policy: PanicPolicy,
        incoming: Vec<ShellEvent>,
        size: (u16, u16),
        hooks: &mut Hooks<'_>,
    ) -> AppResult<()> {
        self.screen.set_dt(self.last.elapsed().as_secs_f32());
        *self.last = std::time::Instant::now();
        profile::begin_frame(profile::BackendStatus::TERMINAL);
        let recovered = frame_isolated(policy, self.screen, self.host, self.router, incoming, size)
            .map_err(Error::io)?;
        if recovered {
            restore();
            self.term.reenter().map_err(Error::io)?;
        }
        if let Some(after) = hooks.after_frame.as_mut() {
            after(self.host, self.router, self.screen);
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

#[allow(
    clippy::too_many_arguments,
    reason = "frame needs the policy, screen, host, router, decoded input, and size"
)]
fn frame_isolated(
    policy: PanicPolicy,
    screen: &mut Screen,
    host: &mut Host,
    router: &mut EventRouter,
    incoming: Vec<ShellEvent>,
    size: (u16, u16),
) -> Result<bool, LayoutError> {
    match policy {
        PanicPolicy::Abort => screen.frame(host, router, incoming, size).map(|_| false),
        PanicPolicy::Isolate => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            screen.frame(host, router, incoming, size).map(|_| false)
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
