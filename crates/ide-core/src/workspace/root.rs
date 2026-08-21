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
#[path = "root_tests.rs"]
mod tests;
