//! Cold-walk perf bench for `WorkspaceRoot::files()`.
//!
//! Target: walk a 10k-file synthetic workspace in ≤ 500ms (release).

#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use crossbeam_channel as _;
use ignore as _;
use notify as _;
use proptest as _;
use ropey as _;
use smallvec as _;
use toml as _;

use std::fs;
use std::path::Path;

use criterion::{Criterion, criterion_group, criterion_main};
use ide_core::workspace::WorkspaceRoot;

fn populate(root: &Path, files: usize, fanout: usize) {
    fs::create_dir_all(root).expect("mk root");
    let mut left = files;
    let mut dir_idx = 0_usize;
    while left > 0 {
        let dir = root.join(format!("d{dir_idx:04}"));
        fs::create_dir_all(&dir).expect("mk subdir");
        let n = left.min(fanout);
        for f in 0..n {
            fs::write(dir.join(format!("f{f:04}.rs")), b"// gen\n").expect("write file");
        }
        left -= n;
        dir_idx += 1;
    }
    // realistic ignore pattern
    fs::write(root.join(".gitignore"), "target\n*.tmp\n").expect("gitignore");
}

fn bench_cold_walk_10k(c: &mut Criterion) {
    let tmp = tempfile::tempdir().expect("tmp");
    populate(tmp.path(), 10_000, 100);
    c.bench_function("workspace_cold_walk_10k", |b| {
        b.iter(|| {
            let root = WorkspaceRoot::open(tmp.path()).expect("open");
            let count = root.files().count();
            assert!(count >= 10_000, "expected ≥10k files, got {count}");
        });
    });
}

criterion_group!(benches, bench_cold_walk_10k);
criterion_main!(benches);
