use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, mpsc};

use serde_json::{Value, json};

use crate::app::embed::Host;
use crate::events::Event;

mod input;
mod query;

use input::events_of;
use query::answer;

const ENV: &str = "SIGIL_INSPECT";
const DEFAULT_ADDR: &str = "127.0.0.1:5761";

type EventQueue = Arc<Mutex<VecDeque<PendingRequest>>>;
use crate::app::Wake;

#[derive(Clone)]
pub(crate) struct Inspect {
    events: EventQueue,
    queries: EventQueue,
}

struct PendingRequest {
    request: Value,
    reply: mpsc::Sender<Value>,
}

#[cfg(feature = "terminal")]
pub(crate) fn install() -> Option<Inspect> {
    bind(None)
}

#[cfg(feature = "desktop")]
pub(crate) fn install_with_wake(wake: impl Fn() + Send + Sync + 'static) -> Option<Inspect> {
    bind(Some(Arc::new(wake) as Wake))
}

fn bind(wake: Option<Wake>) -> Option<Inspect> {
    let value = std::env::var(ENV).ok()?;
    if matches!(value.as_str(), "" | "0" | "false") {
        return None;
    }
    let addr = if value == "1" || value == "true" {
        DEFAULT_ADDR
    } else {
        value.as_str()
    };
    let listener = match TcpListener::bind(addr) {
        Ok(listener) => listener,
        Err(err) => {
            tracing::warn!("sigil-inspect: cannot bind {addr}: {err}");
            return None;
        }
    };
    let inspect = Inspect {
        events: Arc::new(Mutex::new(VecDeque::new())),
        queries: Arc::new(Mutex::new(VecDeque::new())),
    };
    let server = inspect.clone();
    std::thread::Builder::new()
        .name("sigil-inspect".into())
        .spawn(move || server.accept(&listener, wake.as_ref()))
        .ok()?;
    tracing::info!("sigil-inspect: listening on {addr}");
    Some(inspect)
}

impl Inspect {
    #[cfg(feature = "terminal")]
    pub(crate) fn has_pending(&self) -> bool {
        self.events.lock().is_ok_and(|q| !q.is_empty())
            || self.queries.lock().is_ok_and(|q| !q.is_empty())
    }

    fn accept(&self, listener: &TcpListener, wake: Option<&Wake>) {
        for conn in listener.incoming() {
            let Ok(stream) = conn else { continue };
            let (server, wake) = (self.clone(), wake.cloned());
            drop(
                std::thread::Builder::new()
                    .name("sigil-inspect-conn".into())
                    .spawn(move || server.serve(stream, wake.as_ref())),
            );
        }
    }

    fn serve(&self, stream: TcpStream, wake: Option<&Wake>) {
        let Ok(mut writer) = stream.try_clone() else {
            return;
        };
        for line in BufReader::new(stream).lines() {
            let Ok(line) = line else { return };
            if self.handle(&line, &mut writer, wake).is_err() {
                return;
            }
        }
    }

    fn handle(
        &self,
        line: &str,
        writer: &mut TcpStream,
        wake: Option<&Wake>,
    ) -> std::io::Result<()> {
        let request: Value = match serde_json::from_str(line) {
            Ok(request) => request,
            Err(err) => {
                return writeln!(writer, "{}", json!({"ok": false, "error": err.to_string()}));
            }
        };
        let (reply, rx) = mpsc::channel();
        let queue = if input::is_input(&request) {
            &self.events
        } else {
            &self.queries
        };
        drop(
            queue
                .lock()
                .map(|mut q| q.push_back(PendingRequest { request, reply })),
        );
        if let Some(wake) = wake {
            wake();
        }
        let response = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap_or_else(|_| json!({"ok": false, "error": "timeout"}));
        writeln!(writer, "{response}")
    }

    #[cfg(feature = "terminal")]
    pub(crate) fn drain_events(&self, out: &mut Vec<Event>, unit: f32) {
        self.drain(unit, |events| out.extend(events));
    }

    fn drain(&self, unit: f32, mut sink: impl FnMut(Vec<Event>)) {
        while let Some(pending) = pop(&self.events) {
            let response = match events_of(&pending.request, unit) {
                Ok(events) => {
                    sink(events);
                    json!({"ok": true})
                }
                Err(error) => json!({"ok": false, "error": error}),
            };
            drop(pending.reply.send(response));
        }
    }

    pub(crate) fn service(&self, host: &mut Host, mut view: InspectView<'_>) {
        let _env = host.enter();
        self.drain(view.cell.unwrap_or(1.0), |events| {
            for event in events {
                host.route(event);
            }
        });
        while let Some(pending) = pop(&self.queries) {
            drop(
                pending
                    .reply
                    .send(answer(&pending.request, host, &mut view)),
            );
        }
    }
}

pub(crate) type Shot<'a> = dyn FnMut(&mut Host) -> Result<Value, String> + 'a;

pub(crate) struct InspectView<'a> {
    pub(crate) cell: Option<f32>,
    pub(crate) text: Option<&'a str>,
    pub(crate) shot: Option<&'a mut Shot<'a>>,
}

fn pop(queue: &EventQueue) -> Option<PendingRequest> {
    queue.lock().ok().and_then(|mut q| q.pop_front())
}
