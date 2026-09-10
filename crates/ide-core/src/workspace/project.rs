use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

#[derive(Debug, Default, Clone)]
pub struct Project {
    pub source_roots: Vec<PathBuf>,
    pub ignore_patterns: Vec<String>,
    pub languages: BTreeMap<String, LanguageConfig>,
    pub build_commands: BTreeMap<String, String>,
}

#[derive(Debug, Default, Clone)]
pub struct LanguageConfig {
    pub server: Option<String>,
    pub extensions: Vec<String>,
}

impl Project {
    pub fn load(root: &Path) -> io::Result<Option<Self>> {
        let path = root.join(".sigil-ide").join("workspace.toml");
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let table: Table = raw.parse().map_err(|e: toml::de::Error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("parse {}: {e}", path.display()),
            )
        })?;
        Ok(Some(Self::from_table(&table)))
    }

    fn from_table(table: &Table) -> Self {
        Self {
            source_roots: read_path_list(table, "source_roots"),
            ignore_patterns: read_string_list(table, "ignore_patterns"),
            languages: read_languages(table),
            build_commands: read_build_commands(table),
        }
    }
}

fn read_string_list(table: &Table, key: &str) -> Vec<String> {
    let Some(Value::Array(arr)) = table.get(key) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect()
}

fn read_path_list(table: &Table, key: &str) -> Vec<PathBuf> {
    read_string_list(table, key)
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

fn read_languages(table: &Table) -> BTreeMap<String, LanguageConfig> {
    let Some(Value::Table(langs)) = table.get("language") else {
        return BTreeMap::new();
    };
    langs
        .iter()
        .filter_map(|(name, val)| val.as_table().map(|t| (name.clone(), language_from(t))))
        .collect()
}

fn language_from(t: &Table) -> LanguageConfig {
    LanguageConfig {
        server: t.get("server").and_then(|v| v.as_str()).map(str::to_owned),
        extensions: t
            .get("extensions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn read_build_commands(table: &Table) -> BTreeMap<String, String> {
    let Some(Value::Table(build)) = table.get("build") else {
        return BTreeMap::new();
    };
    build
        .iter()
        .filter_map(|(name, val)| val.as_str().map(|s| (name.clone(), s.to_owned())))
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#[path = "project_tests.rs"]
mod tests;
