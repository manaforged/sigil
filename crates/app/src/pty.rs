//! PTY host service. Bind `session.lines` to `terminal()`.

use std::sync::mpsc::{self, Sender};

use sigil_reactive::{Signal, signal};

use crate::{AppResult, Error};

/// Live shell session. Paint is [`Self::lines`]. Bytes to a child
/// need [`Self::spawn`].
#[derive(Clone)]
pub struct Session {
    /// Screen rows, top to bottom.
    pub lines: Signal<Vec<String>>,
    tx: Sender<Vec<u8>>,
}

impl Session {
    /// Empty session. Tests write `lines` directly. [`Self::write`]
    /// errors until [`Self::spawn`].
    #[must_use]
    pub fn new() -> Self {
        let (tx, _rx) = mpsc::channel();
        Self {
            lines: signal(Vec::new()),
            tx,
        }
    }

    /// Send bytes to the PTY child.
    ///
    /// [`Session::new`] has no child. Paint is [`Self::lines`]. Open a
    /// program with [`Self::spawn`].
    ///
    /// # Errors
    ///
    /// Returns when no PTY child is attached.
    pub fn write(&self, bytes: &[u8]) -> AppResult<()> {
        self.tx
            .send(bytes.to_vec())
            .map_err(|_| Error::new(NoChild))
    }

    /// Spawn `program` in a PTY. Requires feature `pty`.
    ///
    /// # Errors
    ///
    /// Returns when the PTY cannot open or the program cannot start.
    /// Without feature `pty`, always errors.
    pub fn spawn(program: &str) -> AppResult<Self> {
        #[cfg(feature = "pty")]
        {
            return spawn_pty(program);
        }
        #[cfg(not(feature = "pty"))]
        {
            let _ = program;
            Err(Error::new(Disabled))
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct NoChild;

impl std::fmt::Display for NoChild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Session::write needs Session::spawn")
    }
}

impl std::error::Error for NoChild {}

#[cfg(not(feature = "pty"))]
#[derive(Debug)]
struct Disabled;

#[cfg(not(feature = "pty"))]
impl std::fmt::Display for Disabled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("enable feature `pty`")
    }
}

#[cfg(not(feature = "pty"))]
impl std::error::Error for Disabled {}

#[cfg(feature = "pty")]
#[derive(Debug)]
struct NoOwner;

#[cfg(feature = "pty")]
impl std::fmt::Display for NoOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Session::spawn needs Runtime::scope")
    }
}

#[cfg(feature = "pty")]
impl std::error::Error for NoOwner {}

#[cfg(feature = "pty")]
fn spawn_pty(program: &str) -> AppResult<Session> {
    spawn_pty_inner(program)
}

#[cfg(feature = "pty")]
fn spawn_pty_inner(program: &str) -> AppResult<Session> {
    use std::io::Write;
    use std::thread;

    use portable_pty::{CommandBuilder, PtySize, native_pty_system};
    use sigil_tasks::{OwnerHandle, dispatcher};

    let owner = OwnerHandle::current().ok_or_else(|| Error::new(NoOwner))?;
    let dispatcher = dispatcher();
    let pty = native_pty_system();
    let pair = pty
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(pty_err)?;
    let mut child = pair
        .slave
        .spawn_command(CommandBuilder::new(program))
        .map_err(pty_err)?;
    let reader = pair.master.try_clone_reader().map_err(pty_err)?;
    let mut writer = pair.master.take_writer().map_err(pty_err)?;
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    thread::spawn(move || {
        while let Ok(bytes) = rx.recv() {
            if writer.write_all(&bytes).is_err() {
                break;
            }
        }
    });
    thread::spawn(move || {
        drop(child.wait());
    });
    let lines = signal(Vec::new());
    let painted = lines.into_sendable();
    thread::spawn(move || read_rows(reader, dispatcher, owner, painted));
    Ok(Session { lines, tx })
}

#[cfg(feature = "pty")]
pub(crate) fn rows_of(parser: &vt100::Parser) -> Vec<String> {
    parser
        .screen()
        .contents()
        .lines()
        .map(str::to_string)
        .collect()
}

#[cfg(feature = "pty")]
fn pty_err(err: impl std::fmt::Display) -> Error {
    Error::new(std::io::Error::other(err.to_string()))
}

#[cfg(feature = "pty")]
fn read_rows(
    mut reader: Box<dyn std::io::Read + Send>,
    dispatcher: sigil_tasks::Dispatcher,
    owner: sigil_tasks::OwnerHandle,
    painted: sigil_reactive::SendableSignal<Vec<String>>,
) {
    use std::io::Read;

    let mut buf = [0_u8; 4096];
    let mut parser = vt100::Parser::new(24, 80, 0);
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let (got, _) = buf.split_at(n);
        parser.process(got);
        let rows = rows_of(&parser);
        sigil_tasks::dispatch_to_owner(&dispatcher, owner, move || {
            painted.into_signal().set(rows);
        });
    }
}

#[cfg(test)]
#[path = "pty_tests.rs"]
mod tests;
