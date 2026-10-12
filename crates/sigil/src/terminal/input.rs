use std::io;
use std::time::Duration;

use crossterm::event::{self, Event};

use crate::events::{ShellEvent, WindowEvent};
use crate::geometry::{LogicalSize, Scale};
use crate::terminal::keys::{map_key, push_key_text, strip_replies};
use crate::terminal::mouse::map_mouse;
use crate::terminal::run::{CELL, Hooks};
use crate::terminal::term::set_focused;
use crate::{AppConfig, AppResult, Error};

pub(crate) enum Incoming {
    Quit,
    Idle,
    Events(Vec<ShellEvent>),
    Resized(u16, u16, Vec<ShellEvent>),
}

pub(crate) struct Wake {
    #[cfg(unix)]
    rx: Option<std::os::unix::net::UnixStream>,
    #[cfg(windows)]
    pasted: std::cell::Cell<Option<std::time::Instant>>,
}

impl Wake {
    #[cfg(unix)]
    pub(crate) fn install(async_rt: &mut crate::tasks::AsyncRuntime) -> Self {
        use std::io::IsTerminal;
        let rx = if io::stdin().is_terminal() {
            wake_pair(async_rt)
        } else {
            None
        };
        Self { rx }
    }

    #[cfg(not(unix))]
    pub(crate) const fn install(_async_rt: &mut crate::tasks::AsyncRuntime) -> Self {
        Self {
            #[cfg(windows)]
            pasted: std::cell::Cell::new(None),
        }
    }

    #[cfg(windows)]
    fn follows_paste(&self) -> bool {
        self.pasted
            .get()
            .is_some_and(|at| at.elapsed() < crate::terminal::keys::PASTE_GAP)
    }

    #[cfg(unix)]
    fn ready(&self, timing: (Option<Duration>, Duration), listen: bool) -> io::Result<bool> {
        self.wait(io::stdin(), timing, listen, event::poll)
    }

    #[cfg(unix)]
    fn wait(
        &self,
        input: impl rustix::fd::AsFd,
        (wait, tick): (Option<Duration>, Duration),
        listen: bool,
        pending: impl Fn(Duration) -> io::Result<bool>,
    ) -> io::Result<bool> {
        use rustix::event::{PollFd, PollFlags, Timespec, poll};
        use std::io::Read;
        if hung_up(&input) {
            return Ok(hang_up());
        }
        let Some(rx) = self
            .rx
            .as_ref()
            .filter(|_| listen && wait.is_none_or(|wait| !wait.is_zero()))
        else {
            return pending(wait.unwrap_or(tick));
        };
        if pending(Duration::ZERO)? {
            return Ok(true);
        }
        let timeout = wait
            .map(Timespec::try_from)
            .transpose()
            .map_err(io::Error::other)?;
        let mut fds = [
            PollFd::new(&input, PollFlags::IN),
            PollFd::new(rx, PollFlags::IN),
        ];
        match poll(&mut fds, timeout.as_ref()) {
            Ok(_) | Err(rustix::io::Errno::INTR) => {}
            Err(errno) => return Err(errno.into()),
        }
        if closed(fds[0].revents()) {
            return Ok(hang_up());
        }
        let mut sink = [0_u8; 64];
        while matches!((&*rx).read(&mut sink), Ok(read) if read > 0) {}
        pending(Duration::ZERO)
    }

    #[cfg(not(unix))]
    #[allow(
        clippy::unused_self,
        reason = "the unix variant drains the wake socket held on self"
    )]
    fn ready(&self, (wait, tick): (Option<Duration>, Duration), _listen: bool) -> io::Result<bool> {
        event::poll(wait.unwrap_or(tick))
    }
}

#[cfg(unix)]
pub(crate) fn hung_up(fd: impl rustix::fd::AsFd) -> bool {
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    let mut fds = [PollFd::new(&fd, PollFlags::IN)];
    let now = Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    poll(&mut fds, Some(&now)).is_ok() && closed(fds[0].revents())
}

#[cfg(unix)]
fn closed(revents: rustix::event::PollFlags) -> bool {
    use rustix::event::PollFlags;
    revents.intersects(PollFlags::HUP | PollFlags::ERR | PollFlags::NVAL)
}

#[cfg(unix)]
fn hang_up() -> bool {
    drop(signal_hook::low_level::raise(signal_hook::consts::SIGHUP));
    false
}

#[cfg(unix)]
fn wake_pair(async_rt: &mut crate::tasks::AsyncRuntime) -> Option<std::os::unix::net::UnixStream> {
    use std::io::Write;
    let (rx, tx) = std::os::unix::net::UnixStream::pair().ok()?;
    rx.set_nonblocking(true).ok()?;
    tx.set_nonblocking(true).ok()?;
    crate::terminal::signals::set_wake(tx.try_clone().ok());
    async_rt.set_waker(move || drop((&tx).write(&[1])));
    Some(rx)
}

pub(crate) fn gather_input(
    first: &mut bool,
    config: &AppConfig,
    pointer: &mut crate::geometry::Point,
    hooks: &mut Hooks<'_>,
    (wake, (wait, listen)): (&Wake, (Option<Duration>, bool)),
) -> AppResult<Incoming> {
    let mut incoming = Vec::new();
    let mut resize = None;
    let fresh = core::mem::take(first);
    let mut ready = !fresh
        && wake
            .ready((wait, config.frame_interval), listen)
            .map_err(Error::io)?;
    let mut burst = Vec::new();
    while ready {
        burst.push(event::read().map_err(Error::io)?);
        ready = event::poll(Duration::ZERO).map_err(Error::io)?;
    }
    strip_replies(&mut burst);
    #[cfg(windows)]
    if let Some(text) = crate::terminal::keys::burst_paste(&burst, wake.follows_paste()) {
        wake.pasted.set(Some(std::time::Instant::now()));
        incoming.push(crate::terminal::keys::paste(&text));
        burst.clear();
    }
    for read in burst {
        match map_input(read, config, pointer) {
            Input::Quit => return Ok(Incoming::Quit),
            Input::Suspend => crate::terminal::signals::stop(),
            Input::Event(mapped) => push_key_text(&mut incoming, mapped),
            Input::Resize(mut cols, mut rows) => {
                limit(config, &mut cols, &mut rows);
                resize = Some((cols, rows));
            }
            Input::None => {}
        }
    }
    if let Some(pump) = hooks.external_events.as_mut() {
        pump(&mut incoming);
    }
    Ok(match resize {
        Some((cols, rows)) => {
            incoming.push(resized(cols, rows));
            Incoming::Resized(cols, rows, incoming)
        }
        None if incoming.is_empty() && !fresh => Incoming::Idle,
        None => Incoming::Events(incoming),
    })
}

enum Input {
    Quit,
    Suspend,
    Event(ShellEvent),
    Resize(u16, u16),
    None,
}

fn map_input(event: Event, config: &AppConfig, pointer: &mut crate::geometry::Point) -> Input {
    match event {
        Event::Key(key) => {
            if quit(&key, config) {
                Input::Quit
            } else {
                map_key(&key).map_or(Input::None, |mapped| keyed(mapped, config))
            }
        }
        Event::Resize(cols, rows) => Input::Resize(cols, rows),
        Event::Mouse(mouse) => map_mouse(&mouse, CELL, pointer).map_or(Input::None, Input::Event),
        Event::Paste(pasted) => Input::Event(crate::terminal::keys::paste(&pasted)),
        Event::FocusGained => focus(true),
        Event::FocusLost => focus(false),
    }
}

fn keyed(mapped: ShellEvent, config: &AppConfig) -> Input {
    if let (ShellEvent::Key(key), Some(shortcut)) = (&mapped, &config.suspend)
        && key.phase == crate::events::KeyPhase::Down
        && shortcut.matches(key)
    {
        Input::Suspend
    } else {
        Input::Event(mapped)
    }
}

fn focus(yes: bool) -> Input {
    set_focused(yes);
    Input::Event(ShellEvent::Window(WindowEvent::Focused(yes)))
}

fn resized(cols: u16, rows: u16) -> ShellEvent {
    let (width, height) = crate::terminal::run::view(cols, rows);
    ShellEvent::Window(WindowEvent::Resized(
        Scale::ONE.to_physical(LogicalSize::new(width, height)),
    ))
}

pub(crate) fn limit(config: &AppConfig, cols: &mut u16, rows: &mut u16) {
    if config.size.0 > 0 {
        *cols = (*cols).min(u16::try_from(config.size.0).unwrap_or(*cols));
    }
    if config.size.1 > 0 {
        *rows = (*rows).min(u16::try_from(config.size.1).unwrap_or(*rows));
    }
}

fn quit(key: &crossterm::event::KeyEvent, config: &AppConfig) -> bool {
    use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
    let ctrl_c = key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c');
    key.kind != KeyEventKind::Release
        && ((config.quit.ctrl_c && ctrl_c) || (config.quit.esc && key.code == KeyCode::Esc))
}

#[cfg(all(test, unix))]
#[path = "input_tests.rs"]
mod tests;
