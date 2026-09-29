use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, mpsc};

use serde_json::{Value, json};

use crate::a11y;
use crate::app::embed::Host;
use crate::events::EventRouter;
use crate::events::ShellEvent;

mod input;

const ENV: &str = "SIGIL_INSPECT";
const DEFAULT_ADDR: &str = "127.0.0.1:5761";

type Queue = Arc<Mutex<VecDeque<Pending>>>;
type Wake = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub(crate) struct Inspect {
    events: Queue,
    queries: Queue,
}

struct Pending {
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

fn is_event(op: &str) -> bool {
    matches!(
        op,
        "click" | "down" | "up" | "move" | "scroll" | "key" | "type"
    )
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
        let queue = match request
            .get("op")
            .and_then(Value::as_str)
            .filter(|op| is_event(op))
        {
            Some(_) => &self.events,
            None => &self.queries,
        };
        drop(
            queue
                .lock()
                .map(|mut q| q.push_back(Pending { request, reply })),
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
    pub(crate) fn drain_events(&self, out: &mut Vec<ShellEvent>, unit: f32) {
        while let Some(pending) = pop(&self.events) {
            let response = match events_of(&pending.request, unit) {
                Ok(events) => {
                    out.extend(events);
                    json!({"ok": true})
                }
                Err(error) => json!({"ok": false, "error": error}),
            };
            drop(pending.reply.send(response));
        }
    }

    pub(crate) fn service(&self, host: &mut Host, router: &mut EventRouter, view: View<'_>) {
        let _env = host.enter();
        let unit = view.cell.unwrap_or(1.0);
        while let Some(pending) = pop(&self.events) {
            let response = match events_of(&pending.request, unit) {
                Ok(events) => {
                    for event in events {
                        host.route(router, event);
                    }
                    json!({"ok": true})
                }
                Err(error) => json!({"ok": false, "error": error}),
            };
            drop(pending.reply.send(response));
        }
        let mut view = view;
        while let Some(pending) = pop(&self.queries) {
            drop(
                pending
                    .reply
                    .send(answer(&pending.request, host, router, &mut view)),
            );
        }
    }
}

pub(crate) type Shot<'a> = dyn FnMut(&mut Host) -> Result<Value, String> + 'a;

pub(crate) struct View<'a> {
    pub(crate) cell: Option<f32>,
    pub(crate) text: Option<&'a str>,
    pub(crate) shot: Option<&'a mut Shot<'a>>,
}

fn pop(queue: &Queue) -> Option<Pending> {
    queue.lock().ok().and_then(|mut q| q.pop_front())
}

fn answer(request: &Value, host: &mut Host, router: &EventRouter, view: &mut View<'_>) -> Value {
    match request.get("op").and_then(Value::as_str) {
        Some("tree") => tree_json(host, router, view.cell),
        Some("snapshot") => {
            json!({"ok": true, "text": a11y::snapshot(&host.accessibility_tree(router))})
        }
        Some("screenshot") => screenshot_json(host, view),
        Some("surfaces") => surfaces_json(host, view.cell),
        Some("settle") => json!({
            "ok": true,
            "settled": !crate::reactive::Runtime::frame_dirty()
                && crate::animation_runtime::active_count() == 0,
        }),
        Some("profile") => profile_json(request),
        Some("profile_dump") => dump_json(request),
        Some("profile_on") => {
            crate::profile::enable();
            json!({"ok": true})
        }
        Some("profile_off") => {
            crate::profile::disable();
            json!({"ok": true})
        }
        Some("hud") => hud_json(request),
        Some(other) => json!({"ok": false, "error": format!("unknown op {other}")}),
        None => json!({"ok": false, "error": "missing op"}),
    }
}

#[allow(clippy::cast_precision_loss, reason = "ns → ms for display")]
fn frame_json(frame: &crate::profile::Frame) -> Value {
    let ms = |ns: u64| Value::from(ns as f64 / 1_000_000.0);
    json!({
        "index": frame.index,
        "cause": frame.cause.name(),
        "wall_ms": ms(frame.wall_ns),
        "phases": frame
            .phases
            .iter()
            .map(|(name, ns)| (name.to_string(), ms(*ns)))
            .collect::<serde_json::Map<String, Value>>(),
        "counts": frame
            .counts
            .iter()
            .map(|(name, value)| (name.to_string(), Value::from(*value)))
            .collect::<serde_json::Map<String, Value>>(),
        "backend": frame.backend.map(|status| status.to_string()),
    })
}

fn profile_json(request: &Value) -> Value {
    let limit = usize::try_from(
        request
            .get("last")
            .and_then(Value::as_u64)
            .unwrap_or(30)
            .min(500),
    )
    .unwrap_or(30);
    let frames = crate::profile::frames();
    let shown: Vec<Value> = frames
        .iter()
        .rev()
        .take(limit)
        .rev()
        .map(frame_json)
        .collect();
    json!({
        "ok": true,
        "enabled": crate::profile::enabled(),
        "frames": shown,
        "summary": crate::profile::summary(),
    })
}

fn dump_json(request: &Value) -> Value {
    let path = request
        .get("path")
        .and_then(Value::as_str)
        .map(std::path::PathBuf::from)
        .or_else(crate::profile::dump_path)
        .unwrap_or_else(|| std::path::PathBuf::from("sigil-profile.json"));
    match crate::profile::dump(&path) {
        Ok(()) => json!({"ok": true, "path": path}),
        Err(error) => json!({"ok": false, "error": error.to_string()}),
    }
}

fn hud_json(request: &Value) -> Value {
    let on = request.get("on").and_then(Value::as_bool).unwrap_or(true);
    crate::hud::set(on);
    json!({"ok": true, "hud": on})
}

fn screenshot_json(host: &mut Host, view: &mut View<'_>) -> Value {
    if let Some(text) = view.text {
        return json!({"ok": true, "text": text});
    }
    view.shot.as_deref_mut().map_or_else(
        || json!({"ok": false, "error": "screenshot is not available on this host"}),
        |shot| match shot(host) {
            Ok(mut body) => {
                body.as_object_mut()
                    .map(|body| body.insert("ok".into(), json!(true)));
                body
            }
            Err(error) => json!({"ok": false, "error": error}),
        },
    )
}

fn surfaces_json(host: &Host, cell: Option<f32>) -> Value {
    let factor = cell.unwrap_or(1.0);
    let surfaces: Vec<Value> = host
        .surfaces()
        .into_iter()
        .map(|(key, r)| {
            json!({
                "key": key,
                "x": r.origin.x / factor,
                "y": r.origin.y / factor,
                "w": r.width / factor,
                "h": r.height / factor,
            })
        })
        .collect();
    json!({"ok": true, "surfaces": surfaces})
}

fn tree_json(host: &mut Host, router: &EventRouter, cell: Option<f32>) -> Value {
    let scale = f64::from(host.cx().scale().factor());
    let update = host.accessibility_tree(router);
    let factor = scale * f64::from(cell.unwrap_or(1.0));
    let nodes: serde_json::Map<String, Value> = update
        .nodes
        .iter()
        .map(|(id, node)| {
            (
                id.0.to_string(),
                node_json(node, *id == update.focus, factor),
            )
        })
        .collect();
    json!({
        "ok": true,
        "root": update.tree.as_ref().map_or(0, |tree| tree.root.0),
        "focus": update.focus.0,
        "nodes": nodes,
    })
}

fn node_json(node: &accesskit::Node, focused: bool, factor: f64) -> Value {
    let mut entry = serde_json::Map::new();
    entry.insert("role".into(), json!(format!("{:?}", node.role())));
    entry.insert(
        "children".into(),
        json!(node.children().iter().map(|c| c.0).collect::<Vec<u64>>()),
    );
    if let Some(name) = node.label() {
        entry.insert("name".into(), json!(name));
    }
    if let Some(desc) = node.description() {
        entry.insert("description".into(), json!(desc));
    }
    if let Some(bounds) = node.bounds() {
        entry.insert(
            "bounds".into(),
            json!([
                bounds.x0 / factor,
                bounds.y0 / factor,
                bounds.x1 / factor,
                bounds.y1 / factor,
            ]),
        );
    }
    if focused {
        entry.insert("focused".into(), json!(true));
    }
    if let Some(sel) = node.is_selected() {
        entry.insert("selected".into(), json!(sel));
    }
    if let Some(exp) = node.is_expanded() {
        entry.insert("expanded".into(), json!(exp));
    }
    if let Some(tog) = node.toggled() {
        entry.insert("toggled".into(), json!(format!("{tog:?}")));
    }
    if let Some(val) = node.value() {
        entry.insert("value".into(), json!(format!("{val:?}")));
    }
    if let Some(num) = node.numeric_value() {
        entry.insert("numeric".into(), json!(num));
    }
    Value::Object(entry)
}

fn events_of(request: &Value, unit: f32) -> Result<Vec<ShellEvent>, String> {
    input::events_of(request, unit)
}
