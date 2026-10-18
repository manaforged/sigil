use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;

use accesskit::{Live, NodeId as AkNodeId, TreeUpdate};

use crate::terminal::paint::Grid;

pub(crate) struct Transcript {
    out: Option<File>,
    tree: String,
    rows: Vec<String>,
    live: Option<HashMap<AkNodeId, String>>,
}

#[derive(Default)]
pub(crate) struct Spot {
    pub(crate) from: (usize, usize),
    pub(crate) to: (usize, usize),
    pub(crate) name: String,
}

#[derive(Default)]
pub(crate) struct Reading {
    pub(crate) focus: Option<Spot>,
    pub(crate) spoken: Vec<String>,
}

impl Transcript {
    pub(crate) fn from_env() -> Self {
        Self {
            out: std::env::var("SIGIL_TUI_TRANSCRIPT")
                .ok()
                .and_then(|value| open(&value)),
            tree: String::new(),
            rows: Vec::new(),
            live: None,
        }
    }

    pub(crate) const fn records(&self) -> bool {
        self.out.is_some()
    }

    pub(crate) fn read(&mut self, update: &TreeUpdate, cell: f32) -> Reading {
        let root = update.tree.as_ref().map(|tree| tree.root);
        let focus = (Some(update.focus) != root)
            .then(|| update.nodes.iter().find(|(id, _)| *id == update.focus))
            .flatten()
            .and_then(|(_, node)| spot(node, cell));
        let mut live = HashMap::new();
        let mut spoken = Vec::new();
        for (id, node) in &update.nodes {
            let Some(label) = node.label().filter(|label| !label.trim().is_empty()) else {
                continue;
            };
            if node.live().is_none_or(|politeness| politeness == Live::Off) {
                continue;
            }
            if let Some(seen) = &self.live
                && seen.get(id).map(String::as_str) != Some(label)
            {
                spoken.push(label.to_string());
            }
            live.insert(*id, label.to_string());
        }
        self.live = Some(live);
        Reading { focus, spoken }
    }

    pub(crate) fn emit(&mut self, update: &TreeUpdate, grid: &Grid) {
        let Some(out) = self.out.as_mut() else {
            return;
        };
        let tree = crate::a11y::snapshot(update);
        let rows = grid.text_rows();
        let written: Vec<std::io::Result<()>> = if tree == self.tree {
            rows.iter()
                .enumerate()
                .filter(|(row, text)| self.rows.get(*row) != Some(*text))
                .map(|(_, text)| writeln!(out, "{text}"))
                .collect()
        } else {
            self.tree = tree;
            vec![writeln!(out, "{}", self.tree).and_then(|()| out.flush())]
        };
        for err in written.into_iter().filter_map(Result::err) {
            tracing::warn!(target: "sigil::terminal", error = %err, "transcript write failed");
        }
        self.rows = rows;
    }
}

fn open(value: &str) -> Option<File> {
    let trimmed = value.trim();
    let path = match trimmed.to_ascii_lowercase().as_str() {
        "" | "0" | "off" | "false" => return None,
        "1" | "on" | "true" => "sigil-transcript.log",
        _ => trimmed,
    };
    OpenOptions::new().create(true).append(true).open(path).ok()
}

impl Grid {
    fn text_rows(&self) -> Vec<String> {
        (0..self.rows())
            .map(|row| {
                self.row(row)
                    .iter()
                    .map(|cell| cell.text.as_str())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }
}

fn spot(node: &accesskit::Node, cell: f32) -> Option<Spot> {
    let rect = node.bounds()?;
    let from = (cell_of(rect.x0, cell), cell_of(rect.y0, cell));
    let to = (
        cell_of(rect.x1, cell).max(from.0 + 1),
        cell_of(rect.y1, cell).max(from.1 + 1),
    );
    let name = node.label().or_else(|| node.value()).unwrap_or_default();
    Some(Spot {
        from,
        to,
        name: name.trim().to_string(),
    })
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "bounds are clamped to zero and floored to a cell index"
)]
fn cell_of(value: f64, cell: f32) -> usize {
    (value.max(0.0) / f64::from(cell)).floor() as usize
}
