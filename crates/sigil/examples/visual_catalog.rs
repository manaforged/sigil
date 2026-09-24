#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::path::PathBuf;
use std::process::ExitCode;

#[path = "visual_catalog/api.rs"]
mod api;
#[path = "visual_catalog/capture.rs"]
mod capture;
#[path = "visual_catalog/catalog.rs"]
mod catalog;
#[path = "visual_catalog/chrome.rs"]
mod chrome;
#[path = "visual_catalog/content.rs"]
mod content;
#[path = "visual_catalog/controls.rs"]
mod controls;
#[path = "visual_catalog/data.rs"]
mod data;
#[path = "visual_catalog/inputs.rs"]
mod inputs;
#[path = "visual_catalog/overlays.rs"]
mod overlays;
#[path = "visual_catalog/plumbing.rs"]
mod plumbing;
#[path = "visual_catalog/registry.rs"]
mod registry;
#[path = "visual_catalog/sheet.rs"]
mod sheet;
#[path = "visual_catalog/shots.rs"]
mod shots;
#[path = "visual_catalog/site.rs"]
mod site;
#[path = "visual_catalog/spec.rs"]
mod spec;
#[path = "visual_catalog/tokens.rs"]
mod tokens;

#[allow(dead_code, unused_attributes)]
#[path = "chat-both.rs"]
mod chat_both;
#[allow(dead_code, unused_attributes)]
#[path = "pomodoro-both.rs"]
mod pomodoro_both;
#[allow(dead_code, unused_attributes)]
#[path = "preferences-window.rs"]
mod preferences_window;
#[allow(dead_code, unused_attributes)]
#[path = "showcase-dashboard.rs"]
mod showcase_dashboard;
#[allow(dead_code, unused_attributes)]
#[path = "showcase-editor.rs"]
mod showcase_editor;
#[allow(dead_code, unused_attributes)]
#[path = "showcase-settings.rs"]
mod showcase_settings;
#[allow(dead_code, unused_attributes)]
#[path = "todo-terminal.rs"]
mod todo_terminal;

fn main() -> ExitCode {
    let Some(out) = std::env::args_os()
        .skip(1)
        .find(|arg| arg != "--site")
        .map(PathBuf::from)
    else {
        eprintln!("usage: visual_catalog --site <output-dir>");
        return ExitCode::FAILURE;
    };
    let mut tasks = match sigil::tasks::AsyncRuntime::new() {
        Ok(tasks) => tasks,
        Err(err) => {
            eprintln!("async runtime: {err}");
            return ExitCode::FAILURE;
        }
    };
    tasks.install_current();
    let result = catalog::load().and_then(|catalog| site::export(&catalog, &out, &mut tasks));
    match result {
        Ok(summary) => {
            println!(
                "{} components, {} states, {} images ({} bytes), manifest {} bytes -> {}",
                summary.components,
                summary.states,
                summary.images,
                summary.bytes,
                summary.manifest,
                out.display()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
