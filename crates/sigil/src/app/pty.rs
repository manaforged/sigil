use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

use crate::events::ModifiersState;
use crate::reactive::{Signal, signal};
use crate::style::{Rgba8, TextRun};
use crate::{AppResult, Error};

mod mouse;
mod reply;
mod view;

pub(crate) use mouse::Mouse;
use reply::{Colors, Replies};

type Parser = vt100::Parser<Replies>;

const SCROLLBACK: usize = 5000;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Row {
    pub(crate) text: String,
    pub(crate) runs: Vec<TextRun>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Frame {
    pub(crate) lines: Vec<String>,
    pub(crate) rows: Vec<Row>,
}

enum Control {
    Resize(u16, u16),
    Scroll(i32),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Modes {
    pub(crate) cursor_keys: bool,
    pub(crate) alternate: bool,
    pub(crate) mouse: vt100::MouseProtocolMode,
    encoding: vt100::MouseProtocolEncoding,
}

/// Child process in a pseudo-terminal (feature `pty`).
pub struct Session {
    /// The lines.
    pub lines: Signal<Vec<String>>,
    /// The title the program set.
    pub title: Signal<String>,
    /// How many times the program rang the bell.
    pub bell: Signal<u64>,
    /// The program's exit code, once it has exited.
    pub exited: Signal<Option<u32>>,
    pub(crate) rows: Signal<Vec<Row>>,
    tx: Sender<Vec<u8>>,
    control: Sender<Control>,
    parser: Arc<Mutex<Parser>>,
    colors: Colors,
    killer: Option<Box<dyn portable_pty::ChildKiller + Send + Sync>>,
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(killer) = self.killer.as_mut()
            && let Err(err) = killer.kill()
        {
            tracing::warn!(target: "sigil::pty", error = %err, "PTY child kill failed");
        }
    }
}

impl Session {
    /// Writes bytes to the terminal session's input.
    pub fn write(&self, bytes: &[u8]) -> AppResult<()> {
        self.tx
            .send(bytes.to_vec())
            .map_err(|_| pty_err("Session::write needs Session::spawn"))
    }

    /// Sends pasted text, bracketed when the program asked for bracketed paste.
    pub fn paste(&self, text: &str) -> AppResult<()> {
        let bracketed = self
            .parser
            .lock()
            .is_ok_and(|parser| parser.screen().bracketed_paste());
        if bracketed {
            self.write(format!("\u{1b}[200~{text}\u{1b}[201~").as_bytes())
        } else {
            self.write(text.as_bytes())
        }
    }

    /// Resizes the terminal to `cols` by `rows` cells.
    pub fn resize(&self, cols: u16, rows: u16) -> AppResult<()> {
        self.control
            .send(Control::Resize(cols.max(1), rows.max(1)))
            .map_err(|_| pty_err("Session::resize after the terminal closed"))
    }

    /// Scrolls the view `rows` into the scrollback; a negative count scrolls back toward the live screen.
    pub fn scroll(&self, rows: i32) -> AppResult<()> {
        self.control
            .send(Control::Scroll(rows))
            .map_err(|_| pty_err("Session::scroll after the terminal closed"))
    }

    pub(crate) fn modes(&self) -> Option<Modes> {
        let parser = self.parser.lock().ok()?;
        let screen = parser.screen();
        Some(Modes {
            cursor_keys: screen.application_cursor(),
            alternate: screen.alternate_screen(),
            mouse: screen.mouse_protocol_mode(),
            encoding: screen.mouse_protocol_encoding(),
        })
    }

    pub(crate) fn report(
        &self,
        event: Mouse,
        at: (u16, u16),
        modifiers: ModifiersState,
        modes: Modes,
    ) -> bool {
        mouse::report(event, at, modifiers, modes.mouse, modes.encoding)
            .is_some_and(|bytes| self.write(&bytes).is_ok())
    }

    pub(crate) fn set_colors(&self, ink: Rgba8, background: Rgba8) {
        if let Ok(mut colors) = self.colors.lock() {
            *colors = (ink, background);
        }
    }

    /// Starts `program` in a new pseudo-terminal.
    pub fn spawn(program: &str) -> AppResult<Self> {
        Self::start(portable_pty::CommandBuilder::new(program), None)
    }

    /// Starts `program` in a new pseudo-terminal with `dir` as its working directory.
    pub fn spawn_in(program: &str, dir: &std::path::Path) -> AppResult<Self> {
        Self::start(portable_pty::CommandBuilder::new(program), Some(dir))
    }

    /// Starts the user's login shell in a new pseudo-terminal with `dir` as its working directory.
    pub fn shell_in(dir: &std::path::Path) -> AppResult<Self> {
        Self::start(portable_pty::CommandBuilder::new_default_prog(), Some(dir))
    }

    fn start(
        mut command: portable_pty::CommandBuilder,
        dir: Option<&std::path::Path>,
    ) -> AppResult<Self> {
        use std::thread;

        use portable_pty::native_pty_system;

        let (owner, dispatcher) = context()?;
        let pair = native_pty_system().openpty(size(80, 24)).map_err(pty_err)?;
        prepare(&mut command);
        if let Some(dir) = dir {
            command.cwd(dir);
        }
        let mut child = pair.slave.spawn_command(command).map_err(pty_err)?;
        let killer = child.clone_killer();
        let reader = pair.master.try_clone_reader().map_err(pty_err)?;
        let tx = input(pair.master.take_writer().map_err(pty_err)?);
        let colors: Colors = Arc::new(Mutex::new(default_colors()));
        let replies = Replies::new(tx.clone(), Arc::clone(&colors));
        let parser = Arc::new(Mutex::new(vt100::Parser::new_with_callbacks(
            24, 80, SCROLLBACK, replies,
        )));
        let published = Published::new();
        let exited = signal(None);
        let publish = Publish::new(dispatcher, owner, Arc::clone(&parser), published);
        let (done, sent) = (publish.clone(), exited.into_sendable());
        thread::spawn(move || {
            let code = child.wait().map_or(1, |status| status.exit_code());
            done.finish(sent, code);
        });
        let (control, orders) = mpsc::channel::<Control>();
        let (master, painter) = (pair.master, publish.clone());
        thread::spawn(move || steer(&orders, master.as_ref(), &painter));
        thread::spawn(move || read_rows(reader, &publish));
        Ok(Self {
            lines: published.lines,
            title: published.title,
            bell: published.bell,
            exited,
            rows: published.rows,
            tx,
            control,
            parser,
            colors,
            killer: Some(killer),
        })
    }
}

fn prepare(command: &mut portable_pty::CommandBuilder) {
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    if cfg!(target_os = "macos") && command.get_env("LANG").is_none() && command.get_env("LC_ALL").is_none() {
        command.env("LANG", "en_US.UTF-8");
    }
}

fn default_colors() -> (Rgba8, Rgba8) {
    crate::style::theme::ambient::ActiveTheme::capture().with(|theme| (theme.on_surface, theme.surface))
}

fn context() -> AppResult<(crate::reactive::Owner, crate::tasks::runtime::Dispatcher)> {
    let owner = crate::reactive::Owner::current()
        .ok_or_else(|| Error::task(std::io::Error::other("Session::spawn needs Owner::scope")))?;
    let dispatcher = crate::tasks::current::dispatcher().ok_or_else(|| {
        Error::task(std::io::Error::other(
            "Session::spawn requires AsyncRuntime::install_current",
        ))
    })?;
    Ok((owner, dispatcher))
}

fn input(mut writer: Box<dyn std::io::Write + Send>) -> Sender<Vec<u8>> {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        while let Ok(bytes) = rx.recv() {
            if writer.write_all(&bytes).is_err() {
                break;
            }
        }
    });
    tx
}

#[derive(Clone, Copy)]
struct Published {
    lines: Signal<Vec<String>>,
    rows: Signal<Vec<Row>>,
    title: Signal<String>,
    bell: Signal<u64>,
}

impl Published {
    fn new() -> Self {
        Self {
            lines: signal(Vec::new()),
            rows: signal(Vec::new()),
            title: signal(String::new()),
            bell: signal(0),
        }
    }
}

#[derive(Clone)]
struct Publish {
    dispatcher: crate::tasks::runtime::Dispatcher,
    owner: crate::reactive::Owner,
    parser: Arc<Mutex<Parser>>,
    lines: crate::reactive::SendableSignal<Vec<String>>,
    rows: crate::reactive::SendableSignal<Vec<Row>>,
    title: crate::reactive::SendableSignal<String>,
    bell: crate::reactive::SendableSignal<u64>,
    queued: Arc<AtomicBool>,
}

impl Publish {
    fn new(
        dispatcher: crate::tasks::runtime::Dispatcher,
        owner: crate::reactive::Owner,
        parser: Arc<Mutex<Parser>>,
        published: Published,
    ) -> Self {
        Self {
            dispatcher,
            owner,
            parser,
            lines: published.lines.into_sendable(),
            rows: published.rows.into_sendable(),
            title: published.title.into_sendable(),
            bell: published.bell.into_sendable(),
            queued: Arc::default(),
        }
    }

    fn send(&self) {
        if self.queued.swap(true, Ordering::AcqRel) {
            return;
        }
        let publish = self.clone();
        crate::tasks::dispatch::dispatch_to_owner(&self.dispatcher, self.owner, move || {
            publish.paint();
        });
    }

    fn paint(&self) {
        self.queued.store(false, Ordering::Release);
        let Ok(mut parser) = self.parser.lock() else {
            return;
        };
        let frame = frame_of(&parser);
        let replies = parser.callbacks_mut();
        let (title, bell) = (replies.title.take(), replies.bell);
        drop(parser);
        self.lines.into_signal().set(frame.lines);
        self.rows.into_signal().set(frame.rows);
        if let Some(title) = title {
            self.title.into_signal().set(title);
        }
        let rung = self.bell.into_signal();
        if rung.peek() != bell {
            rung.set(bell);
        }
    }

    fn finish(&self, exited: crate::reactive::SendableSignal<Option<u32>>, code: u32) {
        crate::tasks::dispatch::dispatch_to_owner(&self.dispatcher, self.owner, move || {
            exited.into_signal().set(Some(code));
        });
    }
}

const fn size(cols: u16, rows: u16) -> portable_pty::PtySize {
    portable_pty::PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

pub(crate) fn rows_of<C: vt100::Callbacks>(parser: &vt100::Parser<C>) -> Vec<String> {
    parser
        .screen()
        .contents()
        .lines()
        .map(str::to_string)
        .collect()
}

pub(crate) fn frame_of<C: vt100::Callbacks>(parser: &vt100::Parser<C>) -> Frame {
    Frame {
        lines: rows_of(parser),
        rows: view::rows(parser.screen()),
    }
}

fn pty_err(err: impl std::fmt::Display) -> Error {
    Error::io(std::io::Error::other(err.to_string()))
}

fn steer(
    orders: &mpsc::Receiver<Control>,
    master: &dyn portable_pty::MasterPty,
    publish: &Publish,
) {
    while let Ok(order) = orders.recv() {
        let Ok(mut parser) = publish.parser.lock() else {
            return;
        };
        match order {
            Control::Resize(cols, rows) => {
                if parser.screen().size() == (rows, cols) {
                    continue;
                }
                parser.screen_mut().set_size(rows, cols);
                if let Err(err) = master.resize(size(cols, rows)) {
                    tracing::warn!(target: "sigil::pty", error = %err, "PTY resize failed");
                }
            }
            Control::Scroll(delta) => {
                let at = parser.screen().scrollback();
                let next = at.saturating_add_signed(isize::try_from(delta).unwrap_or(0));
                parser.screen_mut().set_scrollback(next);
            }
        }
        drop(parser);
        publish.send();
    }
}

fn read_rows(mut reader: Box<dyn std::io::Read + Send>, publish: &Publish) {
    let mut buf = [0_u8; 4096];
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let (got, _) = buf.split_at(n);
        let Ok(mut parser) = publish.parser.lock() else {
            break;
        };
        parser.process(got);
        let synced = parser.callbacks().synced;
        drop(parser);
        if !synced {
            publish.send();
        }
    }
    publish.send();
}

#[cfg(test)]
#[path = "pty_tests.rs"]
mod tests;
