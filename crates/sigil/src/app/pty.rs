use std::sync::mpsc::Sender;

use crate::reactive::Signal;
#[cfg(feature = "pty")]
use crate::reactive::signal;
#[cfg(feature = "pty")]
use std::sync::mpsc;

use crate::app::{AppResult, Error};

pub struct Session {
    pub lines: Signal<Vec<String>>,
    tx: Sender<Vec<u8>>,
    #[cfg(feature = "pty")]
    killer: Option<Box<dyn portable_pty::ChildKiller + Send + Sync>>,
}

impl Drop for Session {
    fn drop(&mut self) {
        #[cfg(feature = "pty")]
        if let Some(killer) = self.killer.as_mut() {
            let _ = killer.kill();
        }
    }
}

impl Session {
    pub fn write(&self, bytes: &[u8]) -> AppResult<()> {
        self.tx
            .send(bytes.to_vec())
            .map_err(|_| Error::new(NoChild))
    }

    pub fn spawn(program: &str) -> AppResult<Self> {
        #[cfg(feature = "pty")]
        let result = spawn_pty(program);
        #[cfg(not(feature = "pty"))]
        let result = {
            let _ = program;
            Err(Error::new(Disabled))
        };
        result
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

    use crate::tasks::{OwnerHandle, dispatcher};
    use portable_pty::{CommandBuilder, PtySize, native_pty_system};

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
    let killer = child.clone_killer();
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
    Ok(Session {
        lines,
        tx,
        killer: Some(killer),
    })
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

#[allow(
    clippy::needless_pass_by_value,
    reason = "the API takes ownership by contract at this boundary"
)]
#[cfg(feature = "pty")]
fn read_rows(
    mut reader: Box<dyn std::io::Read + Send>,
    dispatcher: crate::tasks::Dispatcher,
    owner: crate::tasks::OwnerHandle,
    painted: crate::reactive::SendableSignal<Vec<String>>,
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
        crate::tasks::dispatch_to_owner(&dispatcher, owner, move || {
            painted.into_signal().set(rows);
        });
    }
}

#[cfg(test)]
#[path = "pty_tests.rs"]
mod tests;
