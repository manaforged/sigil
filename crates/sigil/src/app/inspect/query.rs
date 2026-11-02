use serde_json::{Value, json};

use super::InspectView;
use crate::a11y;
use crate::app::embed::Host;

#[derive(Clone, Copy)]
enum InspectQuery {
    Tree,
    Snapshot,
    Screenshot,
    Surfaces,
    Align,
    Settle,
    Profile,
    ProfileDump,
    ProfileOn,
    ProfileOff,
    Hud,
}

const QUERIES: [(&str, InspectQuery); 11] = [
    ("tree", InspectQuery::Tree),
    ("snapshot", InspectQuery::Snapshot),
    ("screenshot", InspectQuery::Screenshot),
    ("surfaces", InspectQuery::Surfaces),
    ("layout::align", InspectQuery::Align),
    ("settle", InspectQuery::Settle),
    ("profile", InspectQuery::Profile),
    ("profile_dump", InspectQuery::ProfileDump),
    ("profile_on", InspectQuery::ProfileOn),
    ("profile_off", InspectQuery::ProfileOff),
    ("hud", InspectQuery::Hud),
];

pub(super) fn answer(request: &Value, host: &mut Host, view: &mut InspectView<'_>) -> Value {
    let Some(name) = request.get("op").and_then(Value::as_str) else {
        return json!({"ok": false, "error": "missing op"});
    };
    let Some(&(_, query)) = QUERIES.iter().find(|(known, _)| *known == name) else {
        return json!({"ok": false, "error": format!("unknown op {name}")});
    };
    match query {
        InspectQuery::Tree => tree_json(host, view.cell),
        InspectQuery::Snapshot => {
            json!({"ok": true, "text": a11y::snapshot(&host.accessibility_tree())})
        }
        InspectQuery::Screenshot => screenshot_json(host, view),
        InspectQuery::Surfaces => surfaces_json(host, view.cell),
        InspectQuery::Align => align_json(host),
        InspectQuery::Settle => json!({
            "ok": true,
            "settled": !crate::reactive::Runtime::frame_dirty()
                && crate::animation_runtime::active_count() == 0,
        }),
        InspectQuery::Profile => profile_json(request),
        InspectQuery::ProfileDump => dump_json(request),
        InspectQuery::ProfileOn => {
            crate::profile::enable();
            json!({"ok": true})
        }
        InspectQuery::ProfileOff => {
            crate::profile::disable();
            json!({"ok": true})
        }
        InspectQuery::Hud => hud_json(request),
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

fn screenshot_json(host: &mut Host, view: &mut InspectView<'_>) -> Value {
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

fn align_json(host: &Host) -> Value {
    use crate::layout::align::{MisalignmentKind, Tolerance};
    let found: Vec<Value> = host
        .misalignments(Tolerance::for_mode(host.host_mode()))
        .iter()
        .map(|item| {
            let kind = match item.kind() {
                MisalignmentKind::Column => "column",
                MisalignmentKind::Inset => "inset",
            };
            json!({"kind": kind, "node": item.node().to_string(), "detail": item.to_string()})
        })
        .collect();
    json!({"ok": true, "misalignments": found})
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

fn tree_json(host: &mut Host, cell: Option<f32>) -> Value {
    let scale = f64::from(host.cx().scale());
    let update = host.accessibility_tree();
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
    let bounds = node
        .bounds()
        .map(|b| json!([b.x0 / factor, b.y0 / factor, b.x1 / factor, b.y1 / factor]));
    let optional = [
        ("name", node.label().map(|name| json!(name))),
        ("description", node.description().map(|desc| json!(desc))),
        ("bounds", bounds),
        ("focused", focused.then_some(json!(true))),
        ("selected", node.is_selected().map(|sel| json!(sel))),
        ("expanded", node.is_expanded().map(|exp| json!(exp))),
        (
            "toggled",
            node.toggled().map(|tog| json!(format!("{tog:?}"))),
        ),
        ("value", node.value().map(|val| json!(format!("{val:?}")))),
        ("numeric", node.numeric_value().map(|num| json!(num))),
    ];
    let mut entry = serde_json::Map::new();
    entry.insert("role".into(), json!(format!("{:?}", node.role())));
    entry.insert(
        "children".into(),
        json!(node.children().iter().map(|c| c.0).collect::<Vec<u64>>()),
    );
    entry.extend(
        optional
            .into_iter()
            .filter_map(|(key, value)| Some((key.to_owned(), value?))),
    );
    Value::Object(entry)
}
