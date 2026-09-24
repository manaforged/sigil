use serde_json::Value;

use super::capture::escape;

const HEAD: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>Sigil visual catalog</title><style>body{font:13px system-ui;margin:16px;background:#f4f4f5}table{border-collapse:collapse}td,th{border:1px solid #d4d4d8;padding:6px;vertical-align:top;background:#fff}th{position:sticky;top:0;background:#e4e4e7}td.name{font-weight:600;white-space:nowrap}figure{display:inline-block;margin:0 6px 6px 0}figcaption{font-size:11px;color:#52525b}.term{background:#18181b;color:#e4e4e7;font:12px/1.2 ui-monospace,monospace;padding:6px;margin:0}</style></head><body><h1>Sigil visual catalog</h1><table><tr><th>Component</th><th>State</th><th>Window</th><th>Terminal</th></tr>";

pub(crate) fn render(manifest: &Value) -> String {
    let mut out = String::from(HEAD);
    for component in list(manifest, "components") {
        let name = text(component, "id");
        for state in list(component, "states") {
            out.extend(["<tr><td class=\"name\">", &escape(name), "</td><td>"]);
            out.push_str(&escape(text(state, "label")));
            out.push_str("</td><td>");
            if let Some(Value::Object(looks)) = state.get("window") {
                for (look, shot) in looks {
                    out.push_str(&figure(look, shot));
                }
            }
            let terminal = state.get("terminal").map_or("", |term| text(term, "dark"));
            out.extend(["</td><td>", terminal, "</td></tr>"]);
        }
    }
    out.push_str("</table></body></html>\n");
    out
}

fn list<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn figure(look: &str, shot: &Value) -> String {
    let width = shot
        .get("width")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    format!(
        "<figure><img src=\"{}\" srcset=\"{} 2x\" width=\"{width}\" alt=\"{look}\"><figcaption>{look}</figcaption></figure>",
        text(shot, "src"),
        text(shot, "src2x"),
    )
}
