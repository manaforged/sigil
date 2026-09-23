use std::io::Write;

use sigil::widgets::{ClipError, Clipboard};

struct Osc52;

impl Clipboard for Osc52 {
    fn set(&self, text: &str) -> Result<(), ClipError> {
        let seq = format!("\x1b]52;c;{}\x07", crate::graphics::base64(text.as_bytes()));
        let mut out = std::io::stdout().lock();
        let written = match crate::graphics::mux() {
            Some(crate::graphics::Mux::Screen) => crate::graphics::write_sequence(&mut out, &seq),
            Some(crate::graphics::Mux::Tmux) | None => out.write_all(seq.as_bytes()),
        };
        written.and_then(|()| out.flush()).map_err(ClipError::new)
    }

    fn get(&self) -> Option<String> {
        None
    }
}

pub fn install_clipboard() {
    sigil::widgets::install(Osc52);
}
