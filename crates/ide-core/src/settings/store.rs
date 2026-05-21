//! [`Store`] — the layered settings primitive.
//!
//! Layout responsibility split: this file owns the layered-lookup logic
//! and the public surface. Path resolution and TOML parsing are inlined
//! here (small enough that splitting into another module would just hide
//! the call). Watch dispatch lives in [`super::watch`].

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::settings::diagnostic::Diagnostic;
use crate::settings::schema::{Entry, Schema};
use crate::settings::value::{ErasedValue, SettingValue};
use crate::settings::watch::{self, Subscription, WatchList};

/// Layered key/value settings store with TOML hot-reload.
///
/// Construct with [`Store::open`]. Read keys with [`Store::get`].
/// Subscribe to changes with [`Store::watch`]. Trigger a re-read with
/// [`Store::reload`].
pub struct Store {
    schema: Schema,
    workspace_path: PathBuf,
    user_path: Option<PathBuf>,
    /// Resolved (post-precedence) value for every schema key. Always
    /// populated for every registered key — either from a TOML layer or
    /// from the schema default.
    resolved: HashMap<String, ErasedValue>,
    /// Per-key watcher list. Lazily inserted on the first
    /// [`Store::watch`] call.
    watchers: Mutex<HashMap<String, WatchList>>,
    next_sub_id: AtomicU64,
}

impl Store {
    /// Open a store rooted at `workspace_root`.
    ///
    /// Reads `<workspace_root>/.sigil-ide/settings.toml` and the user
    /// settings file (platform-specific) synchronously. Missing files
    /// are treated as empty (not an error). Returns the store plus any
    /// non-fatal diagnostics produced during the read.
    pub fn open(workspace_root: &Path, schema: Schema) -> io::Result<(Self, Vec<Diagnostic>)> {
        let workspace_path = workspace_root.join(".sigil-ide").join("settings.toml");
        let user_path = user_settings_path();
        let mut store = Self {
            schema,
            workspace_path,
            user_path,
            resolved: HashMap::new(),
            watchers: Mutex::new(HashMap::new()),
            next_sub_id: AtomicU64::new(1),
        };
        let diags = store.reload_resolved()?;
        Ok((store, diags))
    }

    /// Look up `key`. Returns the resolved value cast to `T`, or `None`
    /// if `T` doesn't match the schema-declared type (i.e. the caller
    /// asked for the wrong type).
    ///
    /// Keys not in the schema always return `None`. Keys in the schema
    /// always have a resolved value — the schema default if no layer set
    /// it.
    #[must_use]
    pub fn get<T: SettingValue>(&self, key: &str) -> Option<T> {
        let erased = self.resolved.get(key)?;
        erased.downcast_ref::<T>().cloned()
    }

    /// Subscribe to changes for `key`. The callback fires whenever the
    /// resolved value for `key` changes after a [`Store::reload`].
    ///
    /// Dropping the returned [`Subscription`] unsubscribes. Callbacks
    /// must not call [`Store::watch`] back for the same store from
    /// inside the callback — doing so deadlocks the per-key watch list.
    pub fn watch<F>(&self, key: &str, on_change: F) -> Subscription
    where
        F: Fn(&Diagnostic) + Send + 'static + Sync,
    {
        let id = self.next_sub_id.fetch_add(1, Ordering::Relaxed);
        let list = self.watcher_list(key);
        if let Ok(mut entries) = list.lock() {
            entries.push(watch::Entry {
                id,
                callback: Box::new(on_change),
            });
        }
        Subscription {
            list: Arc::downgrade(&list),
            id,
        }
    }

    /// Re-read both TOML layers and re-resolve every schema key.
    ///
    /// Callbacks registered via [`Store::watch`] fire for every key
    /// whose resolved value changed. Callbacks fire after the resolve
    /// table is fully updated, so a callback reading [`Store::get`] sees
    /// the new value.
    pub fn reload(&mut self) -> io::Result<Vec<Diagnostic>> {
        let prior = std::mem::take(&mut self.resolved);
        let diags = self.reload_resolved()?;
        let changed = collect_changed_keys(&prior, &self.resolved);
        self.fire_change_callbacks(&changed);
        Ok(diags)
    }

    // ---- internals ----

    fn reload_resolved(&mut self) -> io::Result<Vec<Diagnostic>> {
        let mut diags = Vec::new();
        let workspace_raw = read_toml_layer(&self.workspace_path, &mut diags)?;
        let user_raw = match self.user_path.as_ref() {
            Some(p) => read_toml_layer(p, &mut diags)?,
            None => None,
        };
        for (key, entry) in &self.schema.entries {
            let resolved = resolve_one(
                key,
                entry,
                workspace_raw.as_ref(),
                user_raw.as_ref(),
                &mut diags,
            );
            self.resolved.insert(key.clone(), resolved);
        }
        // Surface unknown-keys for any TOML key not in the schema.
        if let Some(table) = workspace_raw.as_ref() {
            collect_unknown(table, &self.schema, &self.workspace_path, &mut diags);
        }
        if let (Some(table), Some(path)) = (user_raw.as_ref(), self.user_path.as_ref()) {
            collect_unknown(table, &self.schema, path, &mut diags);
        }
        Ok(diags)
    }

    fn watcher_list(&self, key: &str) -> WatchList {
        let mut watchers = match self.watchers.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        watchers
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(Vec::new())))
            .clone()
    }

    fn fire_change_callbacks(&self, changed: &[String]) {
        let snapshot: Vec<(String, WatchList)> = {
            let Ok(watchers) = self.watchers.lock() else {
                return;
            };
            changed
                .iter()
                .filter_map(|k| watchers.get(k).map(|l| (k.clone(), l.clone())))
                .collect()
        };
        for (key, list) in snapshot {
            let diag = Diagnostic::UnknownKey {
                key,
                where_: PathBuf::new(),
            };
            // The `Diagnostic` parameter to user callbacks is a coarse
            // change-notification token rather than a literal error.
            // It carries the key name so the same callback can subscribe
            // to multiple keys and dispatch on `diag.key()`.
            watch::fire(&list, &diag);
        }
    }
}

fn read_toml_layer(path: &Path, diags: &mut Vec<Diagnostic>) -> io::Result<Option<toml::Table>> {
    match std::fs::read_to_string(path) {
        Ok(text) => match toml::from_str::<toml::Table>(&text) {
            Ok(table) => Ok(Some(table)),
            Err(e) => {
                // Surface as a synthetic UnknownKey diagnostic at file
                // scope rather than killing the load. Editors usually
                // want to keep running on a half-broken config.
                diags.push(Diagnostic::UnknownKey {
                    key: format!("<parse error: {e}>"),
                    where_: path.to_path_buf(),
                });
                Ok(None)
            }
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

fn resolve_one(
    key: &str,
    entry: &Entry,
    workspace: Option<&toml::Table>,
    user: Option<&toml::Table>,
    diags: &mut Vec<Diagnostic>,
) -> ErasedValue {
    if let Some(v) = layer_lookup(key, entry, workspace, "workspace", diags) {
        return v;
    }
    if let Some(v) = layer_lookup(key, entry, user, "user", diags) {
        return v;
    }
    entry.default.clone()
}

fn layer_lookup(
    key: &str,
    entry: &Entry,
    layer: Option<&toml::Table>,
    _label: &str,
    diags: &mut Vec<Diagnostic>,
) -> Option<ErasedValue> {
    let raw = lookup_dotted(layer?, key)?;
    let parsed = (entry.parse)(raw);
    if parsed.is_none() {
        diags.push(Diagnostic::TypeMismatch {
            key: key.to_string(),
            expected: entry.type_name,
            found: raw.type_str().to_string(),
            where_: PathBuf::new(),
        });
    }
    parsed
}

fn lookup_dotted<'a>(table: &'a toml::Table, dotted: &str) -> Option<&'a toml::Value> {
    let mut parts = dotted.split('.');
    let first = parts.next()?;
    let mut current = table.get(first)?;
    for part in parts {
        current = current.as_table()?.get(part)?;
    }
    Some(current)
}

fn collect_unknown(table: &toml::Table, schema: &Schema, path: &Path, diags: &mut Vec<Diagnostic>) {
    walk_unknown(table, schema, path, "", diags);
}

fn walk_unknown(
    table: &toml::Table,
    schema: &Schema,
    path: &Path,
    prefix: &str,
    diags: &mut Vec<Diagnostic>,
) {
    for (k, v) in table {
        let full = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        match v {
            toml::Value::Table(t) if !schema.entries.contains_key(&full) => {
                walk_unknown(t, schema, path, &full, diags);
            }
            _ => {
                if !schema.entries.contains_key(&full) {
                    diags.push(Diagnostic::UnknownKey {
                        key: full,
                        where_: path.to_path_buf(),
                    });
                }
            }
        }
    }
}

fn collect_changed_keys(
    prior: &HashMap<String, ErasedValue>,
    current: &HashMap<String, ErasedValue>,
) -> Vec<String> {
    let mut changed = Vec::new();
    for (key, value) in current {
        let differs = prior.get(key).is_none_or(|old| !erased_eq(old, value));
        if differs {
            changed.push(key.clone());
        }
    }
    changed
}

/// Best-effort pointer-equality on `Arc<dyn Any>`. Two resolved values
/// from the same TOML read share an Arc; values built from different
/// layers do not. The corner case (workspace and user TOMLs both set
/// the same key to the same literal in a reload-no-op) would falsely
/// fire callbacks, but the cost is one redundant fire — acceptable.
fn erased_eq(a: &ErasedValue, b: &ErasedValue) -> bool {
    Arc::ptr_eq(a, b) || erased_typed_eq(a, b)
}

fn erased_typed_eq(a: &ErasedValue, b: &ErasedValue) -> bool {
    if let (Some(x), Some(y)) = (a.downcast_ref::<bool>(), b.downcast_ref::<bool>()) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (a.downcast_ref::<i64>(), b.downcast_ref::<i64>()) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (a.downcast_ref::<f64>(), b.downcast_ref::<f64>()) {
        return (x - y).abs() < f64::EPSILON;
    }
    if let (Some(x), Some(y)) = (a.downcast_ref::<String>(), b.downcast_ref::<String>()) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (
        a.downcast_ref::<Vec<String>>(),
        b.downcast_ref::<Vec<String>>(),
    ) {
        return x == y;
    }
    false
}

fn user_settings_path() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    };
    base.map(|b| b.join("sigil-ide").join("settings.toml"))
}
