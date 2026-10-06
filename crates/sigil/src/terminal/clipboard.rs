use std::io::Write;
use std::process::{Command, Stdio};

use crate::clipboard::{ClipError, Clipboard, Delivery};
use crate::terminal::caps::{Caps, ClipRoute};
use crate::terminal::graphics::{Mux, base64, write_mux};

struct Osc52(Caps, std::cell::Cell<bool>);

impl Clipboard for Osc52 {
    fn set(&self, text: &str) -> Result<Delivery, ClipError> {
        if self.0.clipboard == ClipRoute::Pbcopy && pbcopy(text) {
            return Ok(Delivery::Confirmed);
        }
        let seq = format!("\x1b]52;c;{}\x07", base64(text.as_bytes()));
        let mut out = std::io::stdout().lock();
        send(&mut out, &seq, self.0.mux)
            .and_then(|()| out.flush())
            .map(|()| Delivery::Requested)
            .map_err(ClipError::new)
    }

    fn get(&self) -> Option<String> {
        if self.0.clipboard == ClipRoute::Pbcopy {
            return pbpaste();
        }
        if !self.1.get() {
            return None;
        }
        let text = osc52_read(self.0.mux);
        self.1.set(text.is_some());
        text
    }
}

fn send(out: &mut impl Write, seq: &str, mux: Option<Mux>) -> std::io::Result<()> {
    match mux {
        Some(Mux::Screen) => write_mux(out, seq, mux),
        Some(Mux::Tmux) => out
            .write_all(seq.as_bytes())
            .and_then(|()| write_mux(out, seq, mux)),
        None => out.write_all(seq.as_bytes()),
    }
}

#[cfg(unix)]
fn osc52_read(mux: Option<Mux>) -> Option<String> {
    use base64::Engine;
    const REPLY: &str = "\x1b]52;c;";
    if !crate::terminal::term::live() {
        return None;
    }
    let reply = crate::terminal::caps::exchange(
        |out| send(out, "\x1b]52;c;?\x07", mux),
        |text| {
            text.split_once(REPLY)
                .is_some_and(|(_, rest)| rest.contains(['\x07', '\x1b']))
        },
    )?;
    let (_, rest) = reply.split_once(REPLY)?;
    let body = rest.split(['\x07', '\x1b']).next()?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body)
        .ok()?;
    String::from_utf8(bytes).ok()
}

#[cfg(not(unix))]
const fn osc52_read(_mux: Option<Mux>) -> Option<String> {
    None
}

fn pbpaste() -> Option<String> {
    let output = Command::new("/usr/bin/pbpaste")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
}

fn pbcopy(text: &str) -> bool {
    let Ok(mut child) = Command::new("/usr/bin/pbcopy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let written = child
        .stdin
        .take()
        .is_some_and(|mut stdin| stdin.write_all(text.as_bytes()).is_ok());
    child.wait().is_ok_and(|status| status.success()) && written
}

pub fn install_clipboard(caps: Caps) {
    crate::clipboard::install(Osc52(caps, std::cell::Cell::new(true)));
}
