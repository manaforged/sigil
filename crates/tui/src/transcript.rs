use std::fs::{File, OpenOptions};
use std::io::Write;

use sigil::app::Host;
use sigil::events::EventRouter;

use crate::paint::Grid;

pub(crate) struct Transcript {
    out: Option<File>,
    tree: String,
    rows: Vec<String>,
}

impl Transcript {
    pub(crate) fn from_env() -> Self {
        Self {
            out: std::env::var("SIGIL_TUI_TRANSCRIPT")
                .ok()
                .and_then(|value| open(&value)),
            tree: String::new(),
            rows: Vec::new(),
        }
    }

    pub(crate) fn emit(&mut self, host: &Host, router: &EventRouter, grid: &Grid) {
        let Some(out) = self.out.as_mut() else {
            return;
        };
        let tree = sigil::a11y::snapshot(&host.accessibility_tree(router));
        let rows = rows_of(grid);
        if tree != self.tree {
            self.tree = tree;
            self.rows = rows;
            let _ = out.write_all(self.tree.as_bytes());
            let _ = out.write_all(b"\n");
            let _ = out.flush();
            return;
        }
        for (row, text) in rows.iter().enumerate() {
            if self.rows.get(row) != Some(text) {
                let _ = writeln!(out, "{text}");
            }
        }
        self.rows = rows;
    }
}

fn open(value: &str) -> Option<File> {
    let trimmed = value.trim();
    if matches!(
        trimmed.to_ascii_lowercase().as_str(),
        "" | "0" | "off" | "false"
    ) {
        return None;
    }
    let lowered = trimmed.to_ascii_lowercase();
    let path = match lowered.as_str() {
        "1" | "on" | "true" => "sigil-transcript.log",
        _ => trimmed,
    };
    OpenOptions::new().create(true).append(true).open(path).ok()
}

fn rows_of(grid: &Grid) -> Vec<String> {
    (0..grid.rows())
        .map(|row| {
            grid.row(row)
                .iter()
                .map(|cell| cell.text.as_str())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}
