use std::rc::Rc;

use super::parse::{Block, parse_tail};

pub(crate) type KeyedBlocks = Rc<Vec<(u64, Rc<Block>)>>;

#[derive(Default)]
pub(crate) struct Stream {
    prefix: String,
    stable: usize,
    blocks: Vec<(u64, Rc<Block>)>,
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
        let (tail, boundary) = parse_tail(tail_source);
        let old_tail = self.blocks.split_off(self.stable.min(self.blocks.len()));
        for (index, block) in tail.into_iter().enumerate() {
            let key = match old_tail.get(index) {
                Some((key, prev)) if **prev == block => *key,
                _ => {
                    self.next_key += 1;
                    self.next_key
                }
            };
            self.blocks.push((key, Rc::new(block)));
        }
        if let Some((end, count)) = boundary {
            self.prefix
                .push_str(tail_source.get(..end).unwrap_or_default());
            self.stable += count;
        }
        Rc::new(self.blocks.clone())
    }
}
