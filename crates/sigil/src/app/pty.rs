use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

use crate::reactive::{Signal, signal};
use crate::style::TextRun;
use crate::{AppResult, Error};

mod view;

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

/// Child process in a pseudo-terminal (feature `pty`).
pub struct Session {
    /// The lines.
    pub lines: Signal<Vec<String>>,
    pub(crate) rows: Signal<Vec<Row>>,
    tx: Sender<Vec<u8>>,
    control: Sender<Control>,
    parser: Arc<Mutex<vt100::Parser>>,
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

    /// Starts `program` in a new pseudo-terminal.
    pub fn spawn(program: &str) -> AppResult<Self> {
        Self::start(program, None)
    }

    /// Starts `program` in a new pseudo-terminal with `dir` as its working directory.
    pub fn spawn_in(program: &str, dir: &std::path::Path) -> AppResult<Self> {
        Self::start(program, Some(dir))
    }

    fn start(program: &str, dir: Option<&std::path::Path>) -> AppResult<Self> {
        use std::thread;

        use portable_pty::{CommandBuilder, native_pty_system};

        let (owner, dispatcher) = context()?;
        let pair = native_pty_system().openpty(size(80, 24)).map_err(pty_err)?;
        let mut command = CommandBuilder::new(program);
        command.env("TERM", "xterm-256color");
        if let Some(dir) = dir {
            command.cwd(dir);
        }
        let mut child = pair.slave.spawn_command(command).map_err(pty_err)?;
        let killer = child.clone_killer();
        let reader = pair.master.try_clone_reader().map_err(pty_err)?;
        let tx = input(pair.master.take_writer().map_err(pty_err)?);
        thread::spawn(move || {
            drop(child.wait());
        });
        let lines = signal(Vec::new());
        let rows = signal(Vec::new());
        let parser = Arc::new(Mutex::new(vt100::Parser::new(24, 80, SCROLLBACK)));
        let publish = Publish::new(dispatcher, owner, lines, rows);
        let (control, orders) = mpsc::channel::<Control>();
        let (held, master, painter) = (Arc::clone(&parser), pair.master, publish.clone());
        thread::spawn(move || steer(&orders, master.as_ref(), &held, &painter));
        let held = Arc::clone(&parser);
        thread::spawn(move || read_rows(reader, &held, &publish));
        Ok(Self {
            lines,
            rows,
            tx,
            control,
            parser,
            killer: Some(killer),
        })
    }
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

#[derive(Clone)]
struct Publish {
    dispatcher: crate::tasks::runtime::Dispatcher,
    owner: crate::reactive::Owner,
    lines: crate::reactive::SendableSignal<Vec<String>>,
    rows: crate::reactive::SendableSignal<Vec<Row>>,
    pending: Arc<Mutex<Option<Frame>>>,
}

impl Publish {
    fn new(
        dispatcher: crate::tasks::runtime::Dispatcher,
        owner: crate::reactive::Owner,
        lines: Signal<Vec<String>>,
        rows: Signal<Vec<Row>>,
    ) -> Self {
        Self {
            dispatcher,
            owner,
            lines: lines.into_sendable(),
            rows: rows.into_sendable(),
            pending: Arc::default(),
        }
    }

    fn send(&self, parser: &vt100::Parser) -> bool {
        let Ok(mut slot) = self.pending.lock() else {
            return false;
        };
        if slot.replace(frame_of(parser)).is_some() {
            return true;
        }
        drop(slot);
        let (pending, lines, rows) = (Arc::clone(&self.pending), self.lines, self.rows);
        crate::tasks::dispatch::dispatch_to_owner(&self.dispatcher, self.owner, move || {
            if let Some(frame) = pending.lock().ok().and_then(|mut held| held.take()) {
                lines.into_signal().set(frame.lines);
                rows.into_signal().set(frame.rows);
            }
        });
        true
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

pub(crate) fn rows_of(parser: &vt100::Parser) -> Vec<String> {
    parser
        .screen()
        .contents()
        .lines()
        .map(str::to_string)
        .collect()
}

pub(crate) fn frame_of(parser: &vt100::Parser) -> Frame {
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
    parser: &Mutex<vt100::Parser>,
    publish: &Publish,
) {
    while let Ok(order) = orders.recv() {
        let Ok(mut parser) = parser.lock() else {
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
        if !publish.send(&parser) {
            return;
        }
    }
}

fn read_rows(
    mut reader: Box<dyn std::io::Read + Send>,
    parser: &Mutex<vt100::Parser>,
    publish: &Publish,
) {
    let mut buf = [0_u8; 4096];
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let (got, _) = buf.split_at(n);
        let Ok(mut parser) = parser.lock() else {
            break;
        };
        parser.screen_mut().set_scrollback(0);
        parser.process(got);
        if !publish.send(&parser) {
            break;
        }
    }
}

#[cfg(test)]
#[path = "pty_tests.rs"]
mod tests;
