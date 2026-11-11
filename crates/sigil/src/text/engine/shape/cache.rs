use std::cell::RefCell;
use std::hash::{BuildHasher as _, Hash as _, Hasher as _};

use smallvec::SmallVec;

use super::{PushRunArgs, ShapedSegment, segment, u32_clamp};
use crate::cache::Bounded;
use crate::text::glyph::ShapedRun;
use crate::text::shaper::{FaceStyle, Font};

const WORD_ENTRIES: usize = 4_096;

#[derive(Clone)]
pub(super) struct WordSpan {
    pub(super) start: u32,
    pub(super) end: u32,
    pub(super) face_slot: u16,
    pub(super) run: ShapedRun,
    pub(super) advance: f32,
}

struct WordEntry {
    word: Box<str>,
    level: u8,
    faces: SmallVec<[u64; 4]>,
    style: FaceStyle,
    spans: SmallVec<[WordSpan; 1]>,
}

impl WordEntry {
    fn matches(&self, args: &PushRunArgs<'_>, chain: &[Font], style: &FaceStyle) -> bool {
        let same_word = self.word.as_ref() == args.fragment;
        same_word
            && self.level == args.level
            && self.style == *style
            && self.faces.iter().copied().eq(chain.iter().map(Font::id))
    }
}

thread_local! {
    static WORDS: RefCell<Bounded<u64, WordEntry, WORD_ENTRIES>> = RefCell::new(Bounded::default());
}

pub(super) fn replay(
    key: u64,
    args: &PushRunArgs<'_>,
    chain: &[Font],
    style: &FaceStyle,
    out: &mut Vec<ShapedSegment>,
) -> bool {
    WORDS.with_borrow(|words| {
        let entry = words
            .get(&key)
            .filter(|entry| entry.matches(args, chain, style));
        for span in entry.iter().flat_map(|entry| &entry.spans) {
            out.push(segment(args, span.clone()));
        }
        entry.is_some()
    })
}

pub(super) fn remember(
    key: u64,
    args: &PushRunArgs<'_>,
    chain: &[Font],
    style: &FaceStyle,
    shaped: &[ShapedSegment],
) {
    let base = u32_clamp(args.start_byte);
    let spans = shaped
        .iter()
        .map(|seg| WordSpan {
            start: seg.start - base,
            end: seg.end - base,
            face_slot: seg.face_slot,
            run: seg.run.clone(),
            advance: seg.advance,
        })
        .collect();
    WORDS.with_borrow_mut(|words| {
        words.insert(
            key,
            WordEntry {
                word: args.fragment.into(),
                level: args.level,
                faces: chain.iter().map(Font::id).collect(),
                style: style.clone(),
                spans,
            },
        );
    });
}

pub(super) fn word_key(args: &PushRunArgs<'_>, chain: &[Font], style: &FaceStyle) -> u64 {
    let mut hasher = crate::style::text_identity::hasher().build_hasher();
    args.fragment.hash(&mut hasher);
    args.level.hash(&mut hasher);
    for font in chain {
        font.id().hash(&mut hasher);
    }
    style.size_px.to_bits().hash(&mut hasher);
    style.weight.hash(&mut hasher);
    style.letter_spacing_em.to_bits().hash(&mut hasher);
    style.cell_grid.hash(&mut hasher);
    for feature in &style.features {
        feature.0.hash(&mut hasher);
    }
    hasher.finish()
}
