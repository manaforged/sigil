use std::io::Write;
use std::process::{Command, Stdio};

use crate::clipboard::{ClipError, Clipboard, Delivery};
use crate::terminal::caps::{Caps, ClipRoute};
use crate::terminal::graphics::{Mux, base64, write_mux};

struct Osc52(Caps);

impl Clipboard for Osc52 {
    fn set(&self, text: &str) -> Result<Delivery, ClipError> {
        if self.0.clipboard == ClipRoute::Pbcopy && pbcopy(text) {
            return Ok(Delivery::Confirmed);
        }
        let seq = format!("\x1b]52;c;{}\x07", base64(text.as_bytes()));
        let mut out = std::io::stdout().lock();
        let written = match self.0.mux {
            Some(Mux::Screen) => write_mux(&mut out, &seq, self.0.mux),
            Some(Mux::Tmux) => out
                .write_all(seq.as_bytes())
                .and_then(|()| write_mux(&mut out, &seq, self.0.mux)),
            None => out.write_all(seq.as_bytes()),
        };
        written
            .and_then(|()| out.flush())
            .map(|()| Delivery::Requested)
            .map_err(ClipError::new)
    }

    fn get(&self) -> Option<String> {
        None
    }
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
    crate::clipboard::install(Osc52(caps));
}
