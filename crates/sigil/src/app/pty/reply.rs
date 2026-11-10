use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use crate::style::Rgba8;

const SYNCHRONIZED_OUTPUT: u16 = 2026;

pub(super) type Colors = Arc<Mutex<(Rgba8, Rgba8)>>;

pub(super) struct Replies {
    tx: Sender<Vec<u8>>,
    colors: Colors,
    pub(super) title: Option<String>,
    pub(super) bell: u64,
    pub(super) synced: bool,
}

impl Replies {
    pub(super) const fn new(tx: Sender<Vec<u8>>, colors: Colors) -> Self {
        Self {
            tx,
            colors,
            title: None,
            bell: 0,
            synced: false,
        }
    }

    fn send(&self, reply: String) {
        drop(self.tx.send(reply.into_bytes()));
    }

    fn color(&self, slot: &str, pick: fn(&(Rgba8, Rgba8)) -> Rgba8) {
        let Ok(colors) = self.colors.lock() else {
            return;
        };
        let Rgba8 { r, g, b, .. } = pick(&colors);
        self.send(format!(
            "\u{1b}]{slot};rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}\u{1b}\\"
        ));
    }
}

impl vt100::Callbacks for Replies {
    fn audible_bell(&mut self, _: &mut vt100::Screen) {
        self.bell += 1;
    }

    fn set_window_title(&mut self, _: &mut vt100::Screen, title: &[u8]) {
        self.title = Some(String::from_utf8_lossy(title).into_owned());
    }

    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        first: Option<u8>,
        second: Option<u8>,
        params: &[&[u16]],
        end: char,
    ) {
        let param = params
            .first()
            .and_then(|one| one.first())
            .copied()
            .unwrap_or(0);
        let (row, col) = screen.cursor_position();
        let (row, col) = (row + 1, col + 1);
        match (first, second, end, param) {
            (None, None, 'n', 5) => self.send("\u{1b}[0n".to_string()),
            (None, None, 'n', 6) => self.send(format!("\u{1b}[{row};{col}R")),
            (Some(b'?'), None, 'n', 6) => self.send(format!("\u{1b}[?{row};{col};1R")),
            (None, None, 'c', _) => self.send("\u{1b}[?62;22c".to_string()),
            (Some(b'>'), None, 'c', _) => self.send("\u{1b}[>0;10;1c".to_string()),
            (None, None, 't', 18) => {
                let (rows, cols) = screen.size();
                self.send(format!("\u{1b}[8;{rows};{cols}t"));
            }
            (Some(b'?'), None, 'h', SYNCHRONIZED_OUTPUT) => self.synced = true,
            (Some(b'?'), None, 'l', SYNCHRONIZED_OUTPUT) => self.synced = false,
            (Some(b'?'), Some(b'$'), 'p', mode) => {
                let state = match (mode, self.synced) {
                    (SYNCHRONIZED_OUTPUT, true) => 1,
                    (SYNCHRONIZED_OUTPUT, false) => 2,
                    _ => 0,
                };
                self.send(format!("\u{1b}[?{mode};{state}$y"));
            }
            _ => {}
        }
    }

    fn unhandled_osc(&mut self, _: &mut vt100::Screen, params: &[&[u8]]) {
        match params {
            [b"10", b"?"] => self.color("10", |colors| colors.0),
            [b"11", b"?"] => self.color("11", |colors| colors.1),
            _ => {}
        }
    }
}
