use std::sync::mpsc;

use super::{Control, Publish, size};

pub(super) fn steer(
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

pub(super) fn read_rows(mut reader: Box<dyn std::io::Read + Send>, publish: &Publish) {
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
