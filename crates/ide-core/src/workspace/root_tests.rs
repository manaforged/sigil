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
