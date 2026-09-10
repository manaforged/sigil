#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use super::{Diagnostic, Schema, Store};

fn write_workspace_toml(dir: &Path, body: &str) -> PathBuf {
    let cfg_dir = dir.join(".sigil-ide");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    let path = cfg_dir.join("settings.toml");
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn schema_register_default_returned_on_get() {
    let tmp = tempfile::tempdir().unwrap();
    let mut schema = Schema::new();
    schema.register("editor.tab_width", 4_i64);
    let (store, diags) = Store::open(tmp.path(), schema).unwrap();
    assert!(diags.is_empty(), "no toml files = no diags");
    assert_eq!(store.get::<i64>("editor.tab_width"), Some(4));
}

#[test]
fn workspace_toml_overrides_user_toml() {
    let tmp = tempfile::tempdir().unwrap();
    write_workspace_toml(tmp.path(), "tab_width = 8\n");
    let mut schema = Schema::new();
    schema.register("tab_width", 4_i64);
    let (store, _) = Store::open(tmp.path(), schema).unwrap();
    assert_eq!(store.get::<i64>("tab_width"), Some(8));
}

#[test]
fn user_toml_overrides_default() {
    let tmp = tempfile::tempdir().unwrap();
    write_workspace_toml(tmp.path(), "font_size = 14\n");
    let mut schema = Schema::new();
    schema.register("font_size", 12_i64);
    let (store, _) = Store::open(tmp.path(), schema).unwrap();
    assert_eq!(store.get::<i64>("font_size"), Some(14));
}

#[test]
fn unknown_key_emits_diagnostic_not_error() {
    let tmp = tempfile::tempdir().unwrap();
    write_workspace_toml(tmp.path(), "tab_width = 4\nmystery_key = \"hi\"\n");
    let mut schema = Schema::new();
    schema.register("tab_width", 4_i64);
    let (store, diags) = Store::open(tmp.path(), schema).unwrap();
    assert_eq!(store.get::<i64>("tab_width"), Some(4));
    let unknowns = diags
        .iter()
        .filter(|d| matches!(d, Diagnostic::UnknownKey { key, .. } if key == "mystery_key"))
        .count();
    assert_eq!(unknowns, 1, "exactly one UnknownKey for mystery_key");
}

#[test]
fn type_mismatch_emits_diagnostic() {
    let tmp = tempfile::tempdir().unwrap();
    write_workspace_toml(tmp.path(), "tab_width = \"oops\"\n");
    let mut schema = Schema::new();
    schema.register("tab_width", 4_i64);
    let (store, diags) = Store::open(tmp.path(), schema).unwrap();
    assert_eq!(
        store.get::<i64>("tab_width"),
        Some(4),
        "falls back to default"
    );
    let mismatches = diags
        .iter()
        .filter(|d| matches!(d, Diagnostic::TypeMismatch { key, .. } if key == "tab_width"))
        .count();
    assert_eq!(mismatches, 1);
}

#[test]
fn missing_workspace_toml_returns_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let mut schema = Schema::new();
    schema.register("tab_width", 4_i64);
    schema.register("word_wrap", false);
    let (store, diags) = Store::open(tmp.path(), schema).unwrap();
    assert!(diags.is_empty());
    assert_eq!(store.get::<i64>("tab_width"), Some(4));
    assert_eq!(store.get::<bool>("word_wrap"), Some(false));
}

#[test]
fn reload_picks_up_modified_workspace_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_workspace_toml(tmp.path(), "tab_width = 4\n");
    let mut schema = Schema::new();
    schema.register("tab_width", 0_i64);
    let (mut store, _) = Store::open(tmp.path(), schema).unwrap();
    assert_eq!(store.get::<i64>("tab_width"), Some(4));
    std::fs::write(&path, "tab_width = 8\n").unwrap();
    store.reload().unwrap();
    assert_eq!(store.get::<i64>("tab_width"), Some(8));
}

#[test]
fn watch_fires_on_value_change() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_workspace_toml(tmp.path(), "tab_width = 4\n");
    let mut schema = Schema::new();
    schema.register("tab_width", 0_i64);
    let (mut store, _) = Store::open(tmp.path(), schema).unwrap();
    let fired = Arc::new(AtomicUsize::new(0));
    let fired_in = fired.clone();
    let _sub = store.watch("tab_width", move |_| {
        fired_in.fetch_add(1, Ordering::SeqCst);
    });
    std::fs::write(&path, "tab_width = 8\n").unwrap();
    store.reload().unwrap();
    assert_eq!(fired.load(Ordering::SeqCst), 1, "callback fired once");
}

#[test]
fn watch_does_not_fire_when_value_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_workspace_toml(tmp.path(), "tab_width = 4\nfont_size = 12\n");
    let mut schema = Schema::new();
    schema.register("tab_width", 0_i64);
    schema.register("font_size", 0_i64);
    let (mut store, _) = Store::open(tmp.path(), schema).unwrap();
    let fired = Arc::new(AtomicUsize::new(0));
    let fired_in = fired.clone();
    let _sub = store.watch("tab_width", move |_| {
        fired_in.fetch_add(1, Ordering::SeqCst);
    });
    std::fs::write(&path, "tab_width = 4\nfont_size = 16\n").unwrap();
    store.reload().unwrap();
    assert_eq!(
        fired.load(Ordering::SeqCst),
        0,
        "untouched-key callback did not fire"
    );
}

#[test]
fn subscription_drop_unsubscribes() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_workspace_toml(tmp.path(), "tab_width = 4\n");
    let mut schema = Schema::new();
    schema.register("tab_width", 0_i64);
    let (mut store, _) = Store::open(tmp.path(), schema).unwrap();
    let fired = Arc::new(AtomicUsize::new(0));
    {
        let fired_in = fired.clone();
        let _sub = store.watch("tab_width", move |_| {
            fired_in.fetch_add(1, Ordering::SeqCst);
        });
    }
    std::fs::write(&path, "tab_width = 8\n").unwrap();
    store.reload().unwrap();
    assert_eq!(
        fired.load(Ordering::SeqCst),
        0,
        "dropped subscription must not fire"
    );
}

#[test]
fn concurrent_watch_callbacks_thread_safe() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_workspace_toml(tmp.path(), "tab_width = 0\n");
    let mut schema = Schema::new();
    schema.register("tab_width", 0_i64);
    let (mut store, _) = Store::open(tmp.path(), schema).unwrap();
    let fired = Arc::new(AtomicUsize::new(0));
    let store_ref = &store;
    let subs: Vec<_> = (0..8)
        .map(|_| {
            let fired_in = fired.clone();
            store_ref.watch("tab_width", move |_| {
                fired_in.fetch_add(1, Ordering::SeqCst);
            })
        })
        .collect();
    let mut handles = Vec::new();
    for _ in 0..8 {
        let p = tmp.path().to_path_buf();
        let fired_in = fired.clone();
        let mut inner_schema = Schema::new();
        inner_schema.register("tab_width", 0_i64);
        handles.push(thread::spawn(move || {
            let (s, _) = Store::open(&p, inner_schema).unwrap();
            let _sub = s.watch("tab_width", move |_| {
                fired_in.fetch_add(1, Ordering::SeqCst);
            });
        }));
    }
    std::fs::write(&path, "tab_width = 1\n").unwrap();
    store.reload().unwrap();
    for h in handles {
        h.join().unwrap();
    }
    drop(subs);
    assert_eq!(fired.load(Ordering::SeqCst), 8);
}

#[test]
fn non_existent_user_config_dir_is_not_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let mut schema = Schema::new();
    schema.register("tab_width", 4_i64);
    let result = Store::open(tmp.path(), schema);
    assert!(result.is_ok(), "open must not fail when user dir is absent");
}

#[test]
fn dotted_key_resolves_through_nested_table() {
    let tmp = tempfile::tempdir().unwrap();
    write_workspace_toml(tmp.path(), "[editor]\ntab_width = 8\nword_wrap = true\n");
    let mut schema = Schema::new();
    schema.register("editor.tab_width", 4_i64);
    schema.register("editor.word_wrap", false);
    let (store, _) = Store::open(tmp.path(), schema).unwrap();
    assert_eq!(store.get::<i64>("editor.tab_width"), Some(8));
    assert_eq!(store.get::<bool>("editor.word_wrap"), Some(true));
}

proptest::proptest! {
    #[test]
    fn layered_precedence_consistent(ops in proptest::collection::vec(-1000_i64..1000_i64, 1..16)) {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_workspace_toml(tmp.path(), "x = 0\n");
        let mut schema = Schema::new();
        schema.register("x", 999_i64);
        let (mut store, _) = Store::open(tmp.path(), schema).unwrap();
        for n in ops {
            std::fs::write(&path, format!("x = {n}\n")).unwrap();
            store.reload().unwrap();
            assert_eq!(store.get::<i64>("x"), Some(n));
        }
    }
}
