use std::ops::Range;
use std::rc::Rc;

use super::DiffView;
use super::patch::{Hunk, Line, LineId, LineKind, Patch, changed};
use super::rows::GUTTER_W;
use crate::app::cell_width;
use crate::widgets::highlight::language_for_path;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Cell {
    pub(super) id: LineId,
    pub(super) number: usize,
    pub(super) kind: LineKind,
    pub(super) body: String,
    pub(super) range: Option<Range<usize>>,
    pub(super) language: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum DiffRow {
    Meta(String),
    Hunk {
        hunk: usize,
        header: String,
    },
    Split {
        left: Option<Cell>,
        right: Option<Cell>,
        weights: (f32, f32),
    },
    Unified(Cell),
    Overflow,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Model {
    pub(super) rows: Rc<[DiffRow]>,
    pub(super) added: usize,
    pub(super) removed: usize,
    pub(super) weights: (f32, f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PairKind {
    Context,
    Change,
    Note,
}

struct Pair {
    left: Option<usize>,
    right: Option<usize>,
    kind: PairKind,
}

pub(super) fn model(
    patch: &Patch,
    view: DiffView,
    fallback: Option<&'static str>,
    max_rows: usize,
) -> Model {
    let weights = weights(patch);
    let mut rows = Vec::new();
    let (mut added, mut removed) = (0, 0);
    for (index, hunk) in patch.hunks.iter().enumerate() {
        meta_rows(&mut rows, &hunk.meta);
        if !hunk.header.is_empty() {
            rows.push(DiffRow::Hunk {
                hunk: index,
                header: hunk.header.clone(),
            });
        }
        let pairs = pairs(hunk);
        let changes = pairs.iter().filter(|pair| pair.kind == PairKind::Change);
        added += changes.clone().filter(|pair| pair.right.is_some()).count();
        removed += changes.filter(|pair| pair.left.is_some()).count();
        let side = Side {
            hunk: index,
            lines: &hunk.lines,
            language: hunk
                .path
                .as_deref()
                .and_then(language_for_path)
                .or(fallback),
        };
        match view {
            DiffView::Split => rows.extend(pairs.iter().map(|pair| side.split(pair, weights))),
            DiffView::Unified => side.unified(&pairs, &mut rows),
        }
    }
    meta_rows(&mut rows, &patch.trailer);
    if rows.len() > max_rows {
        rows.truncate(max_rows);
        rows.push(DiffRow::Overflow);
    }
    Model {
        rows: rows.into(),
        added,
        removed,
        weights,
    }
}

fn meta_rows(rows: &mut Vec<DiffRow>, meta: &[String]) {
    rows.extend(
        meta.iter()
            .filter(|line| !line.is_empty())
            .map(|line| DiffRow::Meta(line.clone())),
    );
}

fn expand(text: &str) -> String {
    text.replace('\t', "    ")
}

fn weights(patch: &Patch) -> (f32, f32) {
    let lines = || patch.hunks.iter().flat_map(|hunk| hunk.lines.iter());
    let widest = |side: fn(&Line) -> bool| {
        lines()
            .filter(|line| line.kind != LineKind::Note && side(line))
            .map(|line| cell_width(&expand(&line.text)))
            .max()
            .unwrap_or(0)
    };
    let weight =
        |cells: usize| f32::from(u16::try_from(cells).unwrap_or(u16::MAX)) + GUTTER_W / 8.0;
    (
        weight(widest(|line| line.old.is_some())),
        weight(widest(|line| line.new.is_some())),
    )
}

fn pairs(hunk: &Hunk) -> Vec<Pair> {
    let mut out = Vec::new();
    let (mut removed, mut added) = (Vec::new(), Vec::new());
    for (index, line) in hunk.lines.iter().enumerate() {
        let kind = match line.kind {
            LineKind::Removed => {
                removed.push(index);
                continue;
            }
            LineKind::Added => {
                added.push(index);
                continue;
            }
            LineKind::Context => PairKind::Context,
            LineKind::Note => PairKind::Note,
        };
        flush(&hunk.lines, &mut out, &mut removed, &mut added);
        out.push(Pair {
            left: Some(index),
            right: (kind == PairKind::Context).then_some(index),
            kind,
        });
    }
    flush(&hunk.lines, &mut out, &mut removed, &mut added);
    out
}

fn from(list: &[usize], at: usize) -> &[usize] {
    list.get(at..).unwrap_or_default()
}

fn between(list: &[usize], head: usize, tail: usize) -> &[usize] {
    list.get(head..list.len().saturating_sub(tail))
        .unwrap_or_default()
}

fn flush(lines: &[Line], out: &mut Vec<Pair>, dropped: &mut Vec<usize>, kept: &mut Vec<usize>) {
    let text = |index: &usize| lines.get(*index).map(|line| line.text.as_str());
    let same = |(a, b): &(&usize, &usize)| text(a) == text(b);
    let context = |(a, b): (&usize, &usize)| Pair {
        left: Some(*a),
        right: Some(*b),
        kind: PairKind::Context,
    };
    let head = dropped.iter().zip(kept.iter()).take_while(same).count();
    let tail = from(dropped, head)
        .iter()
        .rev()
        .zip(from(kept, head).iter().rev())
        .take_while(same)
        .count();
    out.extend(dropped.iter().zip(kept.iter()).take(head).map(context));
    let left = between(dropped, head, tail);
    let right = between(kept, head, tail);
    out.extend((0..left.len().max(right.len())).map(|index| Pair {
        left: left.get(index).copied(),
        right: right.get(index).copied(),
        kind: PairKind::Change,
    }));
    out.extend(
        from(dropped, dropped.len().saturating_sub(tail))
            .iter()
            .zip(from(kept, kept.len().saturating_sub(tail)).iter())
            .map(context),
    );
    dropped.clear();
    kept.clear();
}

struct Side<'a> {
    hunk: usize,
    lines: &'a [Line],
    language: Option<&'static str>,
}

impl Side<'_> {
    fn text(&self, index: Option<usize>) -> String {
        index
            .and_then(|index| self.lines.get(index))
            .map_or_else(String::new, |line| expand(&line.text))
    }

    fn cell(
        &self,
        index: usize,
        kind: LineKind,
        old: bool,
        range: Option<Range<usize>>,
    ) -> Option<Cell> {
        let line = self.lines.get(index)?;
        Some(Cell {
            id: LineId {
                hunk: self.hunk,
                line: index,
            },
            number: if old { line.old } else { line.new }.unwrap_or(0),
            kind,
            body: expand(&line.text),
            range,
            language: self.language,
        })
    }

    fn ranges(&self, pair: &Pair) -> (Range<usize>, Range<usize>) {
        changed(&self.text(pair.left), &self.text(pair.right))
    }

    fn split(&self, pair: &Pair, weights: (f32, f32)) -> DiffRow {
        let (left_kind, right_kind, ranges) = match pair.kind {
            PairKind::Note => return DiffRow::Meta(self.text(pair.left)),
            PairKind::Context => (LineKind::Context, LineKind::Context, (None, None)),
            PairKind::Change => {
                let (a, b) = self.ranges(pair);
                (LineKind::Removed, LineKind::Added, (Some(a), Some(b)))
            }
        };
        DiffRow::Split {
            left: pair
                .left
                .and_then(|index| self.cell(index, left_kind, true, ranges.0)),
            right: pair
                .right
                .and_then(|index| self.cell(index, right_kind, false, ranges.1)),
            weights,
        }
    }

    fn unified(&self, pairs: &[Pair], rows: &mut Vec<DiffRow>) {
        let mut index = 0;
        while let Some(pair) = pairs.get(index) {
            if pair.kind != PairKind::Change {
                rows.extend(match pair.kind {
                    PairKind::Note => Some(DiffRow::Meta(self.text(pair.left))),
                    _ => pair
                        .right
                        .and_then(|line| self.cell(line, LineKind::Context, false, None))
                        .map(DiffRow::Unified),
                });
                index += 1;
                continue;
            }
            let run = pairs
                .get(index..)
                .unwrap_or_default()
                .iter()
                .take_while(|item| item.kind == PairKind::Change)
                .count();
            let group = pairs.get(index..index + run).unwrap_or_default();
            for kind in [LineKind::Removed, LineKind::Added] {
                rows.extend(group.iter().filter_map(|change| {
                    let (a, b) = self.ranges(change);
                    let (line, old, range) = if kind == LineKind::Removed {
                        (change.left?, true, a)
                    } else {
                        (change.right?, false, b)
                    };
                    self.cell(line, kind, old, Some(range))
                        .map(DiffRow::Unified)
                }));
            }
            index += run;
        }
    }
}
