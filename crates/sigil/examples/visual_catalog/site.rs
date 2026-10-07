use std::panic::AssertUnwindSafe;
use std::path::Path;

use serde_json::{Map, Value, json};
use sigil::layout::LayoutError;
use sigil::tasks::AsyncRuntime;
use sigil_kittest::InteractionMode;

use super::api::{ApiMap, function_source};
use super::builders;
use super::capture::{self, Look, View};
use super::catalog::{Capture, Catalog, Component, Example, HostKind, Live, Settle};
use super::registry;
use super::shots::Shots;
use super::spec::{Build, State};

pub(crate) struct Summary {
    pub(crate) components: usize,
    pub(crate) states: usize,
    pub(crate) images: usize,
    pub(crate) bytes: u64,
    pub(crate) manifest: usize,
}

struct Export<'a> {
    catalog: &'a Catalog,
    out: &'a Path,
    api: ApiMap,
    shots: Shots,
    states: usize,
    tasks: &'a mut AsyncRuntime,
}

pub(crate) fn export(
    catalog: &Catalog,
    out: &Path,
    tasks: &mut AsyncRuntime,
) -> Result<Summary, String> {
    std::fs::create_dir_all(out.join("img")).map_err(|err| format!("create img: {err}"))?;
    let mut export = Export {
        catalog,
        out,
        api: ApiMap::load(),
        shots: Shots::new(),
        states: 0,
        tasks,
    };
    let components = catalog
        .components
        .iter()
        .map(|component| export.component(component))
        .collect::<Result<Vec<_>, _>>()?;
    let examples = catalog
        .examples
        .iter()
        .map(|example| export.example(example))
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = json!({
        "version": 1,
        "crate_version": env!("CARGO_PKG_VERSION"),
        "sigil_commit": commit(),
        "groups": groups(catalog),
        "components": components,
        "examples": examples,
    });
    let text = serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?;
    write(out, "manifest.json", &text)?;
    write(out, "tokens.css", &super::tokens::css()?)?;
    super::tokens::write_fonts(out)?;
    write(out, "index.html", &super::sheet::render(&manifest))?;
    Ok(Summary {
        components: components.len(),
        states: export.states,
        images: export.shots.images,
        bytes: export.shots.bytes,
        manifest: text.len(),
    })
}

fn write(out: &Path, name: &str, text: &str) -> Result<(), String> {
    std::fs::write(out.join(name), text).map_err(|err| format!("write {name}: {err}"))
}

fn groups(catalog: &Catalog) -> Vec<Value> {
    catalog
        .groups
        .iter()
        .map(|group| {
            let members: Vec<&str> = catalog
                .components
                .iter()
                .filter(|component| component.group == group.id)
                .map(|component| component.id.as_str())
                .collect();
            json!({"id": group.id, "title": group.title, "components": members})
        })
        .collect()
}

fn commit() -> String {
    std::env::var("SIGIL_COMMIT")
        .ok()
        .or_else(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
        })
        .map_or_else(|| String::from("unknown"), |sha| sha.trim().to_owned())
}

impl Export<'_> {
    fn component(&mut self, component: &Component) -> Result<Value, String> {
        let group = self
            .catalog
            .groups
            .iter()
            .find(|group| group.id == component.group)
            .ok_or_else(|| format!("{}: unknown group {}", component.id, component.group))?;
        let file = registry::source(&group.source)
            .ok_or_else(|| format!("{}: no source {}", group.id, group.source))?;
        let code = function_source(file, &format!("{}_demo", component.id));
        let summary = self
            .api
            .row(&component.builder)
            .map(|row| row.purpose.clone());
        if code.is_none() || summary.is_none() {
            eprintln!("{}: missing code or api summary", component.id);
        }
        let mut states = Vec::new();
        for capture in &component.captures {
            let build = builders::builder(&capture.build)
                .ok_or_else(|| format!("{}: no builder {}", component.id, capture.build))?;
            for drive in &capture.states {
                states.push(self.state(component, capture, build, *drive)?);
            }
        }
        let live: Vec<Value> = component
            .captures
            .iter()
            .filter(|_| component.live == Live::Sandbox)
            .map(|capture| self.live(capture))
            .collect();
        println!("{}: {} states", component.id, states.len());
        let preview = preview_state(component, &states);
        Ok(json!({
            "id": component.id,
            "title": component.title,
            "group": component.group,
            "builder": component.builder,
            "summary": summary.unwrap_or_default(),
            "preview_state": preview,
            "states": states,
            "live_mode": component.live,
            "live": live,
            "code": code.unwrap_or_default(),
            "api": self.api.rows_for(&component.builder),
        }))
    }

    fn live(&self, capture: &Capture) -> Value {
        let mut drives: Vec<State> = capture.states.clone();
        drives.sort_by_key(|drive| *drive != State::Shown);
        let states: Vec<Value> = drives
            .iter()
            .filter(|drive| **drive == State::Default || built(**drive))
            .filter_map(|drive| {
                let info = self.catalog.state(*drive)?;
                Some(
                    json!({"id": drive, "label": info.label, "capture_id": capture.state_id(info)}),
                )
            })
            .collect();
        json!({
            "build": capture.build,
            "variant": capture.variant,
            "label": capture.label,
            "size": capture.size,
            "states": states,
        })
    }

    fn state(
        &mut self,
        component: &Component,
        capture: &Capture,
        build: Build,
        drive: State,
    ) -> Result<Value, String> {
        let info = self
            .catalog
            .state(drive)
            .ok_or_else(|| format!("{}: no state info for {drive:?}", component.id))?;
        let id = capture.state_id(info);
        let hit = (capture.hit[0], capture.hit[1]);
        let view = View {
            mode: capture::mode(drive, self.catalog.drag),
            size: capture.size,
            pad: self.catalog.pad,
            hit,
        };
        let mut window = Map::new();
        for look in Look::ALL {
            let stem = format!("{}-{id}-{}", component.id, look.tag());
            let shot = self
                .shots
                .pair(self.out, &stem, self.catalog.crop_pad, |scale| {
                    capture::paint(move || build(drive), look, view, scale)
                });
            window.insert(look.tag().to_owned(), logged(&stem, shot));
        }
        let tree = move || build(drive);
        let cells = capture.cells(self.catalog.cell);
        let focus = drive == State::Focused;
        let mut terminal = Map::new();
        for look in Look::ALL {
            let run = capture::Run {
                look,
                events: capture::drive(drive, hit, self.catalog.drag),
                focus,
                cells,
                settle: Settle::default(),
                tasks: &mut *self.tasks,
            };
            if let Some(term) = terminal_grid(|| capture::grid(tree, run), look) {
                terminal.insert("cols".into(), json!(term.cols));
                terminal.insert("rows".into(), json!(term.rows));
                terminal.insert(look.tag().into(), json!(term.html));
            }
        }
        self.states += 1;
        Ok(json!({
            "id": id,
            "label": capture.state_label(info),
            "window": window,
            "terminal": terminal,
        }))
    }

    fn example(&mut self, example: &Example) -> Result<Value, String> {
        let app = registry::app(&example.build)
            .ok_or_else(|| format!("{}: no app {}", example.id, example.build))?;
        let source = registry::source(&example.source)
            .ok_or_else(|| format!("{}: no source {}", example.id, example.source))?;
        let mut shots = Vec::new();
        if example.hosts.contains(&HostKind::Window) {
            self.window_shots(example, app, &mut shots);
        }
        if example.hosts.contains(&HostKind::Terminal) {
            self.terminal_shots(example, app, &mut shots);
        }
        println!("{}: {} shots", example.id, shots.len());
        Ok(json!({
            "id": example.id,
            "title": example.title,
            "summary": example.summary,
            "hosts": example.hosts,
            "shots": shots,
            "source_path": example.source,
            "source": source,
        }))
    }
}

impl Export<'_> {
    fn window_shots(&mut self, example: &Example, app: registry::App, shots: &mut Vec<Value>) {
        let view = View {
            mode: InteractionMode::Default,
            size: example.size,
            pad: 0.0,
            hit: (0.0, 0.0),
        };
        for look in Look::ALL {
            let stem = format!("{}-window-{}", example.id, look.tag());
            let shot = self
                .shots
                .pair(self.out, &stem, self.catalog.crop_pad, |scale| {
                    capture::paint(app, look, view, scale)
                });
            if let Value::Object(mut fields) = logged(&stem, shot) {
                fields.insert("host".into(), json!(HostKind::Window));
                fields.insert("theme".into(), json!(look.tag()));
                shots.push(Value::Object(fields));
            }
        }
    }

    fn terminal_shots(&mut self, example: &Example, app: registry::App, shots: &mut Vec<Value>) {
        for look in Look::ALL {
            let run = capture::Run {
                look,
                events: Vec::new(),
                focus: false,
                cells: example.grid.into(),
                settle: example.settle,
                tasks: &mut *self.tasks,
            };
            if let Some(term) = terminal_grid(|| capture::grid(app, run), look) {
                shots.push(json!({
                    "host": HostKind::Terminal,
                    "theme": look.tag(),
                    "terminal_html": term.html,
                    "cols": term.cols,
                    "rows": term.rows,
                }));
            }
        }
    }
}

fn preview_state(component: &Component, states: &[Value]) -> Value {
    let ids: Vec<&str> = states
        .iter()
        .filter_map(|state| state.get("id").and_then(Value::as_str))
        .collect();
    let chosen = component
        .preview
        .as_deref()
        .or_else(|| ids.first().copied());
    if let Some(id) = chosen.filter(|id| !ids.contains(id)) {
        eprintln!("{}: preview state {id} is not captured", component.id);
    }
    chosen.map_or(Value::Null, |id| json!(id))
}

fn logged(stem: &str, shot: Result<Value, String>) -> Value {
    shot.unwrap_or_else(|err| {
        eprintln!("{stem}: {err}");
        Value::Null
    })
}

fn terminal_grid(
    run: impl FnOnce() -> Result<capture::Cells, LayoutError>,
    look: Look,
) -> Option<capture::Term> {
    match std::panic::catch_unwind(AssertUnwindSafe(run)) {
        Ok(Ok(cells)) => Some(capture::term(&cells, look)),
        Ok(Err(err)) => {
            eprintln!("terminal layout: {err}");
            None
        }
        Err(_) => {
            eprintln!("terminal capture panicked");
            None
        }
    }
}

const fn built(state: State) -> bool {
    matches!(
        state,
        State::Disabled | State::Invalid | State::Selected | State::Shown
    )
}
