use std::io;
use std::path::{Path, PathBuf};

use ignore::overrides::OverrideBuilder;
use ignore::{DirEntry, Walk, WalkBuilder};

use super::project::Project;
use super::watch::Watcher;

#[derive(Debug)]
pub struct WorkspaceRoot {
    path: PathBuf,
    project: Option<Project>,
}

impl WorkspaceRoot {
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

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub const fn project(&self) -> Option<&Project> {
        self.project.as_ref()
    }

    #[must_use]
    pub fn files(&self) -> WalkIter {
        let walk = self.build_walker().build();
        WalkIter { inner: walk }
    }

    #[must_use]
    pub fn entries(&self) -> EntryIter {
        let walk = self.build_walker().build();
        let root = self.path.clone();
        EntryIter { inner: walk, root }
    }

    pub fn watch(&self) -> io::Result<Watcher> {
        Watcher::start(&self.path)
    }

    fn build_walker(&self) -> WalkBuilder {
        let mut builder = WalkBuilder::new(&self.path);
        builder.require_git(false);
        if let Some(project) = &self.project
            && !project.ignore_patterns.is_empty()
        {
            let mut overrides = OverrideBuilder::new(&self.path);
            for pat in &project.ignore_patterns {
                let inverted = format!("!{pat}");
                let _ = overrides.add(&inverted);
            }
            if let Ok(set) = overrides.build() {
                builder.overrides(set);
            }
        }
        builder
    }
}

#[allow(missing_debug_implementations)]
pub struct WalkIter {
    inner: Walk,
}

impl Iterator for WalkIter {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = self.inner.next()?.ok()?;
            if is_file(&entry) {
                return Some(entry.into_path());
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkEntry {
    pub path: PathBuf,
    pub is_dir: bool,
}

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
#[path = "root_tests.rs"]
mod tests;
