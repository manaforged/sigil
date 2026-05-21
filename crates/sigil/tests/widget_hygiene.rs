//! Phase 0.7 — hygiene enforcement over the widget public surface.
//!
//! Scans `crates/sigil/src/widgets/**/*.rs` and fails the build if any
//! forbidden naming pattern reappears. Documents the W4.99 + Phase 0
//! ergonomics bar so future dispatches can't quietly regress it.
//!
//! Forbidden patterns:
//!
//! - `pub fn on_<name>` — event-handler builder methods must be one-word
//!   (Phase 0.1; e.g. `.activate()`, `.change()`, `.submit()`).
//! - `pub fn with_<name>` — no `with_` builder prefix (W4.99 bar).
//! - `pub fn set_<name>` / `pub fn get_<name>` / `pub fn create_<name>` —
//!   configure-as-noun rule; setters and getters drop the verb prefix.
//! - `pub fn <name>_px(` — no `_px` suffix on layout setters (Phase 0.2;
//!   pixel values infer through `impl Into<Dimension>` etc.).
//! - `pub fn fixed_size(` — Wave A item 4; use `.width().height()` instead.
//! - `pub fn style_atoms(` — Wave B item 9; the builder setter is `.style()`,
//!   the trait surface is `.style_mut()`.
//! - `pub fn open(... Signal<bool>)` on modal — Wave A item 3; use `.show()`.

#![allow(dead_code, unused_crate_dependencies, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn widget_surface_obeys_ergonomics_bar() {
    let widgets = widget_root();
    let mut files = Vec::new();
    collect_rs_files(&widgets, &mut files);
    assert!(
        !files.is_empty(),
        "no widget files found under {widgets:?}; test wiring is broken"
    );

    let mut violations: Vec<String> = Vec::new();
    for path in &files {
        let source = fs::read_to_string(path).expect("read widget source");
        for (lineno, line) in source.lines().enumerate() {
            check_line(path, lineno + 1, line, &mut violations);
        }
    }

    assert!(
        violations.is_empty(),
        "Phase 0.7 hygiene gate failed — widget surface drifted from the ergonomics bar:\n\n{}\n\nSee crates/sigil/tests/widget_hygiene.rs for the rule list.",
        violations.join("\n")
    );
}

fn widget_root() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest).join("src").join("widgets")
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn check_line(path: &Path, lineno: usize, line: &str, out: &mut Vec<String>) {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("pub fn ") && !trimmed.starts_with("pub(crate) fn ") {
        return;
    }
    let after_pub_fn = trimmed
        .strip_prefix("pub fn ")
        .or_else(|| trimmed.strip_prefix("pub(crate) fn "))
        .unwrap_or(trimmed);

    // Drop generics + arg list — we only care about the leading identifier.
    let name_end = after_pub_fn
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(after_pub_fn.len());
    let name = &after_pub_fn[..name_end];

    let rel = path
        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path)
        .display();
    let cite = format!("  {rel}:{lineno}  pub fn {name}");

    if name.starts_with("on_") {
        out.push(format!("{cite}  — forbidden `on_` prefix (Phase 0.1)"));
    }
    if name.starts_with("with_") {
        out.push(format!("{cite}  — forbidden `with_` prefix (W4.99 bar)"));
    }
    if name.starts_with("set_") {
        out.push(format!(
            "{cite}  — forbidden `set_` prefix (configure-as-noun)"
        ));
    }
    if name.starts_with("get_") {
        out.push(format!(
            "{cite}  — forbidden `get_` prefix (drop verb on getters)"
        ));
    }
    if name.starts_with("create_") {
        out.push(format!("{cite}  — forbidden `create_` prefix"));
    }
    if name.ends_with("_px") {
        out.push(format!("{cite}  — forbidden `_px` suffix (Phase 0.2)"));
    }
    if name == "fixed_size" {
        out.push(format!(
            "{cite}  — forbidden `fixed_size`; use .width().height() (Wave A item 4)"
        ));
    }
    if name == "style_atoms" {
        out.push(format!("{cite}  — forbidden `style_atoms`; setter is .style(), trait surface is .style_mut() (Wave B item 9)"));
    }
}
