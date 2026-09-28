use std::path::{Path, PathBuf};

use super::{FileFilter, PickKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum EntryKind {
    Parent,
    Dir,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) kind: EntryKind,
    pub(super) name: String,
    pub(super) path: PathBuf,
}

impl Entry {
    pub(super) fn title(&self) -> String {
        match self.kind {
            EntryKind::Parent => String::from(".."),
            EntryKind::Dir => format!("{}/", self.name),
            EntryKind::File => self.name.clone(),
        }
    }
}

pub(super) fn start_dir(directory: Option<&Path>) -> PathBuf {
    directory
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("/"))
}

pub(super) fn list(dir: &Path, kind: PickKind, filters: &[FileFilter]) -> Vec<Entry> {
    let mut entries: Vec<Entry> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|item| {
            let path = item.path();
            let entry_kind = if path.is_dir() {
                EntryKind::Dir
            } else {
                EntryKind::File
            };
            let name = item.file_name().to_string_lossy().into_owned();
            Entry {
                kind: entry_kind,
                name,
                path,
            }
        })
        .filter(|entry| shown(entry, kind, filters))
        .collect();
    entries.sort_by_key(|entry| (entry.kind, entry.name.to_lowercase()));
    if let Some(parent) = dir.parent() {
        entries.insert(
            0,
            Entry {
                kind: EntryKind::Parent,
                name: String::from(".."),
                path: parent.to_path_buf(),
            },
        );
    }
    entries
}

fn shown(entry: &Entry, kind: PickKind, filters: &[FileFilter]) -> bool {
    match entry.kind {
        EntryKind::Parent | EntryKind::Dir => true,
        EntryKind::File => kind != PickKind::Folder && matches_filters(&entry.path, filters),
    }
}

fn matches_filters(path: &Path, filters: &[FileFilter]) -> bool {
    if filters.is_empty() {
        return true;
    }
    let ext = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase());
    filters
        .iter()
        .flat_map(|filter| &filter.extensions)
        .any(|allowed| allowed == "*" || ext.as_deref() == Some(allowed.to_lowercase().as_str()))
}
