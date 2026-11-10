use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use super::{Control, Parser, Row, frame_of, size};
use crate::reactive::{Signal, signal};

#[derive(Clone, Copy)]
pub(super) struct Published {
    pub(super) lines: Signal<Vec<String>>,
    pub(super) rows: Signal<Vec<Row>>,
    pub(super) title: Signal<String>,
    pub(super) bell: Signal<u64>,
}

impl Published {
    pub(super) fn new() -> Self {
        Self {
            lines: signal(Vec::new()),
            rows: signal(Vec::new()),
            title: signal(String::new()),
            bell: signal(0),
        }
    }
}

#[derive(Clone)]
pub(super) struct Publish {
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
    pub(super) fn new(
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

    pub(super) fn finish(&self, exited: crate::reactive::SendableSignal<Option<u32>>, code: u32) {
        crate::tasks::dispatch::dispatch_to_owner(&self.dispatcher, self.owner, move || {
            exited.into_signal().set(Some(code));
        });
    }
}

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
