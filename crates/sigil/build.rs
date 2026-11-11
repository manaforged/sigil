use std::env;
use std::fmt::Write as _;
use std::io::{self, Write as _};

fn feature(name: &str) -> bool {
    env::var_os(format!(
        "CARGO_FEATURE_{}",
        name.to_uppercase().replace('-', "_")
    ))
    .is_some()
}

fn flag(name: &str) -> bool {
    env::var_os(format!("CARGO_CFG_{name}")).is_some()
}

fn target(key: &str, value: &str) -> bool {
    env::var(format!("CARGO_CFG_{key}"))
        .is_ok_and(|values| values.split(',').any(|item| item == value))
}

fn aliases() -> Vec<(&'static str, bool)> {
    let kittest = feature("kittest");
    let desktop = feature("desktop");
    let terminal = feature("terminal");
    let scene = feature("scene");
    let inspect = feature("inspect");
    let wasm = target("TARGET_ARCH", "wasm32");
    let macos = target("TARGET_OS", "macos");
    let unix = flag("UNIX");

    let app_host = desktop || terminal;
    let host = kittest || app_host;
    let scene_harness = scene && kittest;
    let native_commands = feature("menu") || feature("media");
    let native_events = native_commands || feature("hotkey");
    let vendor_os = macos || target("TARGET_OS", "windows");
    let native_http = feature("image-http") && !wasm;

    vec![
        ("app_host", app_host),
        ("host", host),
        ("terminal_host", kittest || terminal),
        ("window_host", kittest || desktop),
        ("scene_host", scene && host),
        ("scene_paint", kittest || (scene && app_host)),
        ("scene_harness", scene_harness),
        ("scene_internal", scene && !kittest),
        ("cpu_paint", kittest || (scene && terminal)),
        ("scene_emit", app_host || scene_harness),
        ("vector_paint", scene || terminal),
        ("raster", scene || kittest),
        ("capture", inspect || kittest),
        ("inspector", inspect && app_host),
        ("desktop_harness", desktop && kittest),
        ("live_desktop", desktop && !kittest),
        ("native_desktop", desktop && !wasm),
        ("native_commands", native_commands),
        ("native_events", native_events),
        ("event_channel", kittest || native_events),
        ("hot_patch", feature("hot") && !wasm),
        ("global_hotkey", feature("hotkey") && !wasm),
        ("vega_js", feature("vega") && !wasm),
        ("native_http", native_http),
        ("http_fetch", wasm || native_http),
        ("font_listeners", wasm || host),
        ("vendor_os", vendor_os),
        (
            "os_notifications",
            vendor_os || (unix && !target("TARGET_OS", "android")),
        ),
        ("freedesktop", unix && !macos && !wasm),
        ("fuzz_replies", flag("FUZZING") && unix),
    ]
}

fn main() -> io::Result<()> {
    let mut out = String::from("cargo::rerun-if-changed=build.rs\n");
    for (name, on) in aliases() {
        let _ = writeln!(out, "cargo::rustc-check-cfg=cfg({name})");
        if on {
            let _ = writeln!(out, "cargo::rustc-cfg={name}");
        }
    }
    io::stdout().write_all(out.as_bytes())
}
