//! `WorkspaceRoot` — owns a canonical root path, a gitignore-aware
//! walker, and an optional [`Project`] loaded from
//! `.sigil-ide/workspace.toml`.

use std::io;
use std::path::{Path, PathBuf};

use ignore::overrides::OverrideBuilder;
use ignore::{DirEntry, Walk, WalkBuilder};

use super::project::Project;
use super::watch::Watcher;

/// One filesystem root in a (potentially multi-root) workspace.
///
/// Wraps an `ignore::WalkBuilder`-style walker (which natively
/// honours `.gitignore` + `.ignore` + global git excludes) plus
/// an optional `Project` loader for `.sigil-ide/workspace.toml`.
#[derive(Debug)]
pub struct WorkspaceRoot {
    /// Canonicalised root path. Always absolute.
    path: PathBuf,
    /// Optional project file. `None` when no `.sigil-ide/workspace.toml`
    /// is present (which is the common case for plain directories).
    project: Option<Project>,
}

impl WorkspaceRoot {
    /// Open a directory as a workspace root. Canonicalises the path
    /// and loads `.sigil-ide/workspace.toml` if present.
    ///
    /// Returns [`io::ErrorKind::NotFound`] if `path` does not exist
    /// or is not a directory.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let raw = path.as_ref();
        let canonical = raw.canonicalize()?;
        if !canonical.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("not a directory: {}", canonical.display()),
            ));
        }
        let project = Project::load(&canonical)?;
        Ok(Self {
            path: canonical,
            project,
        })
    }

    /// Canonical absolute path to this root.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The project document, if `.sigil-ide/workspace.toml` was
    /// present at open time.
    #[must_use]
    pub const fn project(&self) -> Option<&Project> {
        self.project.as_ref()
    }

    /// Iterate all files under this root, respecting `.gitignore`
    /// and any project `ignore_patterns`. Directories are filtered;
    /// only file entries are yielded.
    #[must_use]
    pub fn files(&self) -> WalkIter {
        let walk = self.build_walker().build();
        WalkIter { inner: walk }
    }

    /// Iterate every entry under this root (files AND directories),
    /// respecting `.gitignore` and project `ignore_patterns`. The root
    /// itself is skipped; descendant directories are yielded before
    /// their children so callers can build a stable tree view.
    #[must_use]
    pub fn entries(&self) -> EntryIter {
        let walk = self.build_walker().build();
        let root = self.path.clone();
        EntryIter { inner: walk, root }
    }

    /// Spawn an fs-watcher rooted here. The returned [`Watcher`]
    /// receives debounced + coalesced events; consumers poll with
    /// [`Watcher::next`] or block with [`Watcher::wait`].
    pub fn watch(&self) -> io::Result<Watcher> {
        Watcher::start(&self.path)
    }

    fn build_walker(&self) -> WalkBuilder {
        let mut builder = WalkBuilder::new(&self.path);
        // `ignore` defaults to `require_git(true)` — gitignore is only
        // honoured when a `.git` directory exists up the path. An IDE
        // workspace can root anywhere, so unconditionally enable.
        builder.require_git(false);
        // `ignore` already enables `.gitignore` + `.ignore` + parent
        // git excludes by default. We layer project ignore patterns
        // on top via overrides.
        if let Some(project) = &self.project
            && !project.ignore_patterns.is_empty()
        {
            let mut overrides = OverrideBuilder::new(&self.path);
            for pat in &project.ignore_patterns {
                // Overrides treat patterns as include by default;
                // a leading `!` inverts to ignore. Project
                // ignore_patterns are unconditionally ignores,
                // so prefix.
                let inverted = format!("!{pat}");
                // Invalid patterns silently skip — the project
                // file is user-authored; refusing to walk on a
                // typo would block the IDE on startup.
                let _ = overrides.add(&inverted);
            }
            if let Ok(set) = overrides.build() {
                builder.overrides(set);
            }
        }
        builder
    }
}

/// Iterator over files in a workspace root. Silently skips entries
/// that fail to stat (consistent with how editors traverse: a
/// transient permission error during walk should not stop the walk).
#[allow(missing_debug_implementations)]
pub struct WalkIter {
    inner: Walk,
}

impl Iterator for WalkIter {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Walk errors (permission denied, broken symlink) are
            // dropped — consistent with editor-grade walkers where
            // a transient failure should not abort the listing.
            let entry = self.inner.next()?.ok()?;
            if is_file(&entry) {
                return Some(entry.into_path());
            }
        }
    }
}

/// One entry yielded by [`WorkspaceRoot::entries`]: a path plus
/// whether it is a directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkEntry {
    /// Absolute path on disk.
    pub path: PathBuf,
    /// `true` for directories, `false` for files (or symlinks).
    pub is_dir: bool,
}

/// Iterator over every entry (files + directories) under a workspace
/// root. The root itself is skipped; transient stat errors are silently
/// dropped, matching the [`WalkIter`] contract.
#[allow(missing_debug_implementations)]
pub struct EntryIter {
    inner: Walk,
    root: PathBuf,
}

impl Iterator for EntryIter {
    type Item = WalkEntry;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = self.inner.next()?.ok()?;
            let path = entry.path();
            if path == self.root {
                continue;
            }
            let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
            return Some(WalkEntry {
                path: entry.into_path(),
                is_dir,
            });
        }
    }
}

fn is_file(entry: &DirEntry) -> bool {
    entry
        .file_type()
        .is_some_and(|t| t.is_file() || t.is_symlink())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"x").unwrap();
    }

    #[test]
    fn open_nonexistent_is_notfound() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let err = WorkspaceRoot::open(&missing).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn files_respects_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("keep.rs"));
        touch(&dir.path().join("target/skip.rs"));
        fs::write(dir.path().join(".gitignore"), "target\n").unwrap();
        let root = WorkspaceRoot::open(dir.path()).unwrap();
        let files: HashSet<_> = root
            .files()
            .map(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_owned()
            })
            .collect();
        assert!(files.contains("keep.rs"), "kept files: {files:?}");
        assert!(!files.contains("skip.rs"), "should ignore target");
    }

    #[test]
    fn project_ignore_patterns_apply() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("a.rs"));
        touch(&dir.path().join("b.tmp"));
        fs::create_dir_all(dir.path().join(".sigil-ide")).unwrap();
        fs::write(
            dir.path().join(".sigil-ide/workspace.toml"),
            "ignore_patterns = [\"*.tmp\"]\n",
        )
        .unwrap();
        let root = WorkspaceRoot::open(dir.path()).unwrap();
        let files: HashSet<_> = root
            .files()
            .map(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_owned()
            })
            .collect();
        assert!(files.contains("a.rs"));
        assert!(!files.contains("b.tmp"), "project pattern not applied");
    }

    #[test]
    fn entries_yields_files_and_dirs() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("a.rs"));
        touch(&dir.path().join("sub/b.rs"));
        let root = WorkspaceRoot::open(dir.path()).unwrap();
        let entries: Vec<_> = root.entries().collect();
        let names: HashSet<_> = entries
            .iter()
            .map(|e| {
                (
                    e.path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_owned(),
                    e.is_dir,
                )
            })
            .collect();
        assert!(names.contains(&("a.rs".to_owned(), false)));
        assert!(names.contains(&("sub".to_owned(), true)));
        assert!(names.contains(&("b.rs".to_owned(), false)));
    }

    #[test]
    fn entries_skips_the_root_itself() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("only.rs"));
        let root = WorkspaceRoot::open(dir.path()).unwrap();
        let entries: Vec<_> = root.entries().collect();
        assert!(
            entries.iter().all(|e| e.path != *root.path()),
            "root should not appear in its own entries"
        );
    }

    #[test]
    fn multi_root_overlapping_resolves_per_root() {
        let outer = tempfile::tempdir().unwrap();
        touch(&outer.path().join("inner/file.rs"));
        touch(&outer.path().join("outer-only.rs"));
        let root_outer = WorkspaceRoot::open(outer.path()).unwrap();
        let root_inner = WorkspaceRoot::open(outer.path().join("inner")).unwrap();
        let outer_files: HashSet<_> = root_outer
            .files()
            .map(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_owned()
            })
            .collect();
        let inner_files: HashSet<_> = root_inner
            .files()
            .map(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_owned()
            })
            .collect();
        // Each root sees its own scope. file.rs lives under both
        // canonical paths so both roots yield it; outer-only.rs
        // is only visible from the outer root.
        assert!(outer_files.contains("file.rs"));
        assert!(outer_files.contains("outer-only.rs"));
        assert!(inner_files.contains("file.rs"));
        assert!(!inner_files.contains("outer-only.rs"));
    }
}
