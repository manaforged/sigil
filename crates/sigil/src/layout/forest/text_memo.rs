#![allow(clippy::redundant_pub_crate)]

use std::hash::{Hash as _, Hasher as _};

use ahash::AHasher;

use super::{LayoutText, LocalId};
use crate::kernel::TextContent;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(super) struct TextMemoKey {
    pub(super) index: u32,
    pub(super) tag: u32,
    pub(super) known_w: u32,
    pub(super) known_h: u32,
    pub(super) wrap: u32,
    pub(super) content: u64,
}

pub(super) fn text_memo_key(
    id: LocalId,
    known: crate::layout::style_data::Size<Option<f32>>,
    wrap_to: Option<f32>,
    text: &LayoutText,
) -> TextMemoKey {
    TextMemoKey {
        index: id.slot(),
        tag: id.era(),
        known_w: known.width.map_or(u32::MAX, f32::to_bits),
        known_h: known.height.map_or(u32::MAX, f32::to_bits),
        wrap: wrap_to.map_or(u32::MAX, f32::to_bits),
        content: text_memo_content(text),
    }
}

fn text_memo_content(text: &LayoutText) -> u64 {
    let mut hasher = AHasher::default();
    match &text.content {
        TextContent::Static(value) => value.hash(&mut hasher),
        live => live.resolve().hash(&mut hasher),
    }
    text.wrap.hash(&mut hasher);
    text.ellipsis.hash(&mut hasher);
    if let Some(style) = &text.style {
        crate::layout::text_style::text_style_signature(style).hash(&mut hasher);
    }
    hasher.finish()
}

pub(super) const TEXT_MEMO_LIVE: usize = 8_192;

#[derive(Default)]
pub(super) struct TextMemo {
    pub(super) live: ahash::AHashMap<TextMemoKey, crate::layout::style_data::Size<f32>>,
    pub(super) stale: ahash::AHashMap<TextMemoKey, crate::layout::style_data::Size<f32>>,
}

impl TextMemo {
    pub(super) fn get(&mut self, key: TextMemoKey) -> Option<crate::layout::style_data::Size<f32>> {
        if let Some(hit) = self.live.get(&key) {
            return Some(*hit);
        }
        let hit = self.stale.remove(&key)?;
        self.live.insert(key, hit);
        Some(hit)
    }

    pub(super) fn store(&mut self, key: TextMemoKey, size: crate::layout::style_data::Size<f32>) {
        if self.live.len() >= TEXT_MEMO_LIVE {
            self.stale = std::mem::take(&mut self.live);
        }
        self.live.insert(key, size);
    }

    fn clear(&mut self) {
        self.live.clear();
        self.stale.clear();
    }
}

pub(super) fn text_memo_get(key: TextMemoKey) -> Option<crate::layout::style_data::Size<f32>> {
    TEXT_MEMO.with(|cell| cell.borrow_mut().get(key))
}

pub(super) fn text_memo_store(key: TextMemoKey, size: crate::layout::style_data::Size<f32>) {
    TEXT_MEMO.with(|cell| cell.borrow_mut().store(key, size));
}

pub(crate) fn text_memo_clear() {
    TEXT_MEMO.with(|cell| cell.borrow_mut().clear());
}

thread_local! {
    static TEXT_MEMO: std::cell::RefCell<TextMemo> = std::cell::RefCell::new(TextMemo::default());
}
