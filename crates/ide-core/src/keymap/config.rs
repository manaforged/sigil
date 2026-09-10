use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf};

use super::key::{Key, ParseError};
use super::resolver::Bindings;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::exhaustive_enums)]
pub enum KeymapDiagnostic {
    ParseError {
        path: PathBuf,
        message: String,
    },
    InvalidKey {
        path: PathBuf,
        raw: String,
        cause: ParseError,
    },
    InvalidValue {
        path: PathBuf,
        raw: String,
        reason: &'static str,
    },
    InvalidContext {
        path: PathBuf,
        raw: String,
    },
}

impl KeymapDiagnostic {
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::ParseError { path, .. }
            | Self::InvalidKey { path, .. }
            | Self::InvalidValue { path, .. }
            | Self::InvalidContext { path, .. } => path,
        }
    }
}

pub struct Keymap {
    defaults: Bindings,
    user: Bindings,
    workspace: Bindings,
    flattened: Bindings,
    workspace_path: PathBuf,
    user_path: Option<PathBuf>,
}

impl Keymap {
    pub fn open(
        workspace_root: &Path,
        defaults: Bindings,
    ) -> io::Result<(Self, Vec<KeymapDiagnostic>)> {
        let workspace_path = workspace_root.join(".sigil-ide").join("keymap.toml");
        let user_path = user_keymap_path();
        let mut diags = Vec::new();
        let user = match user_path.as_ref() {
            Some(p) => read_layer(p, &mut diags)?,
            None => Bindings::new(),
        };
        let workspace = read_layer(&workspace_path, &mut diags)?;
        let flattened = flatten(&defaults, &user, &workspace);
        Ok((
            Self {
                defaults,
                user,
                workspace,
                flattened,
                workspace_path,
                user_path,
            },
            diags,
        ))
    }

    #[must_use]
    pub const fn bindings(&self) -> &Bindings {
        &self.flattened
    }

    #[must_use]
    pub const fn defaults(&self) -> &Bindings {
        &self.defaults
    }

    #[must_use]
    pub const fn user(&self) -> &Bindings {
        &self.user
    }

    #[must_use]
    pub const fn workspace(&self) -> &Bindings {
        &self.workspace
    }

    pub fn reload(&mut self) -> io::Result<Vec<KeymapDiagnostic>> {
        let mut diags = Vec::new();
        self.user = match self.user_path.as_ref() {
            Some(p) => read_layer(p, &mut diags)?,
            None => Bindings::new(),
        };
        self.workspace = read_layer(&self.workspace_path, &mut diags)?;
        self.flattened = flatten(&self.defaults, &self.user, &self.workspace);
        Ok(diags)
    }
}

fn flatten(defaults: &Bindings, user: &Bindings, workspace: &Bindings) -> Bindings {
    let mut out = Bindings::new();
    for b in defaults.iter() {
        out.insert(
            b.keys.clone(),
            b.action.clone(),
            b.required_context.iter().cloned(),
        );
    }
    for b in user.iter() {
        out.insert(
            b.keys.clone(),
            b.action.clone(),
            b.required_context.iter().cloned(),
        );
    }
    for b in workspace.iter() {
        out.insert(
            b.keys.clone(),
            b.action.clone(),
            b.required_context.iter().cloned(),
        );
    }
    out
}

fn read_layer(path: &Path, diags: &mut Vec<KeymapDiagnostic>) -> io::Result<Bindings> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Bindings::new()),
        Err(e) => return Err(e),
    };
    let table: toml::Table = match toml::from_str(&text) {
        Ok(t) => t,
        Err(e) => {
            diags.push(KeymapDiagnostic::ParseError {
                path: path.to_path_buf(),
                message: e.to_string(),
            });
            return Ok(Bindings::new());
        }
    };
    let mut bindings = Bindings::new();
    let Some(section) = table.get("bindings").and_then(toml::Value::as_table) else {
        return Ok(bindings);
    };
    for (raw_keys, value) in section {
        ingest_entry(path, raw_keys, value, &mut bindings, diags);
    }
    Ok(bindings)
}

fn ingest_entry(
    path: &Path,
    raw_keys: &str,
    value: &toml::Value,
    bindings: &mut Bindings,
    diags: &mut Vec<KeymapDiagnostic>,
) {
    let keys = match parse_chord(raw_keys) {
        Ok(k) => k,
        Err(cause) => {
            diags.push(KeymapDiagnostic::InvalidKey {
                path: path.to_path_buf(),
                raw: raw_keys.to_string(),
                cause,
            });
            return;
        }
    };
    let (action, contexts) = match value {
        toml::Value::String(s) => (s.clone(), Vec::new()),
        toml::Value::Table(t) => {
            if let Some(pair) = extract_table_form(t) {
                pair
            } else {
                diags.push(KeymapDiagnostic::InvalidValue {
                    path: path.to_path_buf(),
                    raw: raw_keys.to_string(),
                    reason: "table form requires string `action`",
                });
                return;
            }
        }
        _ => {
            diags.push(KeymapDiagnostic::InvalidValue {
                path: path.to_path_buf(),
                raw: raw_keys.to_string(),
                reason: "expected string or table",
            });
            return;
        }
    };
    if let toml::Value::Table(t) = value
        && let Some(when) = t.get("when")
        && !when_is_string_array(when)
    {
        diags.push(KeymapDiagnostic::InvalidContext {
            path: path.to_path_buf(),
            raw: raw_keys.to_string(),
        });
        return;
    }
    let owned_contexts: Vec<Cow<'static, str>> = contexts.into_iter().map(Cow::Owned).collect();
    bindings.insert(keys, Cow::Owned(action), owned_contexts);
}

fn extract_table_form(t: &toml::Table) -> Option<(String, Vec<String>)> {
    let action = t.get("action")?.as_str()?.to_string();
    let contexts = match t.get("when") {
        None => Vec::new(),
        Some(toml::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Some(_) => return Some((action, Vec::new())),
    };
    Some((action, contexts))
}

fn when_is_string_array(v: &toml::Value) -> bool {
    match v {
        toml::Value::Array(arr) => arr.iter().all(toml::Value::is_str),
        _ => false,
    }
}

fn parse_chord(s: &str) -> Result<Vec<Key>, ParseError> {
    let mut out = Vec::new();
    for token in s.split_whitespace() {
        out.push(Key::parse(token)?);
    }
    if out.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(out)
}

fn user_keymap_path() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    };
    base.map(|b| b.join("sigil-ide").join("keymap.toml"))
}
