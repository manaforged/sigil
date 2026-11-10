use std::path::{Path, PathBuf};

use super::Stream;
use crate::widgets::markdown::parse::Block;

const STEPS: [usize; 3] = [1, 7, 61];
const MAX_UPDATES: usize = 2000;

fn documents(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            documents(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
}

fn blocks(stream: &mut Stream, source: &str) -> Vec<Block> {
    stream
        .update(source)
        .iter()
        .map(|(_, block)| (**block).clone())
        .collect()
}

fn streamed(source: &str, step: usize) -> Vec<Block> {
    let step = step.max(source.len() / MAX_UPDATES);
    let mut stream = Stream::default();
    let mut end = 0;
    while end < source.len() {
        end = (end + step).min(source.len());
        while !source.is_char_boundary(end) {
            end += 1;
        }
        blocks(&mut stream, &source[..end]);
    }
    blocks(&mut stream, source)
}

#[test]
fn every_repo_document_streams_to_its_full_parse() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut paths = vec![root.join("README.md")];
    documents(&root.join("docs"), &mut paths);
    assert!(paths.len() > 10, "corpus missing under {}", root.display());
    let mut failures = Vec::new();
    for path in &paths {
        let source = std::fs::read_to_string(path).expect("corpus file is UTF-8");
        let full = blocks(&mut Stream::default(), &source);
        for step in STEPS {
            if streamed(&source, step) != full {
                failures.push(format!("{} at step {step}", path.display()));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "streamed parse differs:\n{}",
        failures.join("\n")
    );
}
