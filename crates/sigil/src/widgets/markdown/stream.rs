use std::collections::HashSet;
use std::rc::Rc;

use super::parse::{Block, Tail, parse_tail};
use super::refs::Refs;
use crate::widgets::text_search::find_ci;

pub(crate) type KeyedBlocks = Rc<Vec<(u64, Rc<Block>)>>;

#[derive(Default)]
pub(crate) struct Stream {
    prefix: String,
    stable: usize,
    refs: Refs,
    notes_seen: HashSet<String>,
    blocks: Vec<(u64, Rc<Block>)>,
    notes: Option<(u64, Rc<Block>)>,
    next_key: u64,
}

impl Stream {
    pub(crate) fn update(&mut self, source: &str) -> KeyedBlocks {
        if crate::widgets::source_too_large(source) || !source.starts_with(&self.prefix) {
            self.reset();
        }
        let mut tail = self.parse(source);
        if self.stale(&tail) {
            self.reset();
            tail = self.parse(source);
            self.stale(&tail);
        }
        self.apply(tail, source);
        Rc::new(self.ordered())
    }

    fn reset(&mut self) {
        self.prefix.clear();
        self.stable = 0;
        self.refs = Refs::default();
        self.notes_seen.clear();
    }

    fn parse(&self, source: &str) -> Tail {
        let tail_source = source.get(self.prefix.len()..).unwrap_or_default();
        parse_tail(&super::repair::tail(tail_source), &self.refs)
    }

    fn stale(&mut self, tail: &Tail) -> bool {
        let mut stale = self.refs.wanted_by(&tail.report);
        for block in &tail.blocks {
            let Block::Footnotes(notes) = block else {
                continue;
            };
            for note in notes {
                if self.notes_seen.insert(note.label.clone()) {
                    let mark = format!("[^{}]", note.label);
                    stale |= find_ci(&self.prefix, &mark).is_some();
                }
            }
        }
        stale
    }

    fn apply(&mut self, tail: Tail, source: &str) {
        let old_tail = self.blocks.split_off(self.stable.min(self.blocks.len()));
        for (index, block) in tail.blocks.into_iter().enumerate() {
            let key = match old_tail.get(index) {
                Some((key, prev)) if **prev == block => *key,
                _ => self.fresh_key(),
            };
            self.blocks.push((key, Rc::new(block)));
        }
        if let Some((end, count)) = tail.cut {
            let start = self.prefix.len();
            self.refs.commit(&tail.report, end);
            self.prefix
                .push_str(source.get(start..start + end).unwrap_or_default());
            self.stable += count;
        }
    }

    const fn fresh_key(&mut self) -> u64 {
        self.next_key += 1;
        self.next_key
    }

    fn ordered(&mut self) -> Vec<(u64, Rc<Block>)> {
        let mut out = Vec::with_capacity(self.blocks.len() + 1);
        let mut notes = Vec::new();
        for (key, block) in &self.blocks {
            match &**block {
                Block::Footnotes(group) => notes.extend(group.iter().cloned()),
                _ => out.push((*key, Rc::clone(block))),
            }
        }
        if notes.is_empty() {
            self.notes = None;
            return out;
        }
        let merged = Block::Footnotes(notes);
        let entry = match self.notes.take() {
            Some((key, prev)) if *prev == merged => (key, prev),
            _ => (self.fresh_key(), Rc::new(merged)),
        };
        out.push((entry.0, Rc::clone(&entry.1)));
        self.notes = Some(entry);
        out
    }
}


#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
