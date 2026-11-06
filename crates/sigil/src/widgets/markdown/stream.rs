use std::rc::Rc;

use super::parse::{Block, parse_tail};

pub(crate) type KeyedBlocks = Rc<Vec<(u64, Rc<Block>)>>;

#[derive(Default)]
pub(crate) struct Stream {
    prefix: String,
    stable: usize,
    blocks: Vec<(u64, Rc<Block>)>,
    notes: Option<(u64, Rc<Block>)>,
    next_key: u64,
}

impl Stream {
    pub(crate) fn update(&mut self, source: &str) -> KeyedBlocks {
        if crate::widgets::source_too_large(source) || !source.starts_with(&self.prefix) {
            self.prefix.clear();
            self.stable = 0;
        }
        let start = self.prefix.len();
        let tail_source = source.get(start..).unwrap_or_default();
        let (tail, boundary) = parse_tail(&super::repair::tail(tail_source));
        let old_tail = self.blocks.split_off(self.stable.min(self.blocks.len()));
        for (index, block) in tail.into_iter().enumerate() {
            let key = match old_tail.get(index) {
                Some((key, prev)) if **prev == block => *key,
                _ => self.fresh_key(),
            };
            self.blocks.push((key, Rc::new(block)));
        }
        if let Some((end, count)) = boundary {
            self.prefix
                .push_str(tail_source.get(..end).unwrap_or_default());
            self.stable += count;
        }
        Rc::new(self.ordered())
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
