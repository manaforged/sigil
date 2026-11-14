use std::cell::RefCell;
use std::hash::{BuildHasher as _, Hash as _, Hasher as _};

use smallvec::SmallVec;

use super::{PushRunArgs, ShapedSegment, segment, u32_clamp};
use crate::cache::Bounded;
use crate::text::glyph::ShapedRun;
use crate::text::shaper::{FaceStyle, Font};

const WORD_ENTRIES: usize = 4_096;

pub(in crate::text::engine) type WordCache = Bounded<u64, WordEntry, WORD_ENTRIES>;

pub(in crate::text::engine) struct SpanKey {
    pub(super) hash: u64,
    faces: SmallVec<[u64; 8]>,
}

impl SpanKey {
    pub(in crate::text::engine) fn new(chain: &[Font], style: &FaceStyle) -> Self {
        let faces: SmallVec<[u64; 8]> = chain.iter().map(Font::id).collect();
        let mut hasher = crate::style::text_identity::hasher().build_hasher();
        for &face in &faces {
            face.hash(&mut hasher);
        }
        style.size_px.to_bits().hash(&mut hasher);
        style.weight.hash(&mut hasher);
        style.letter_spacing_em.to_bits().hash(&mut hasher);
        style.cell_grid.hash(&mut hasher);
        style.font.hash(&mut hasher);
        Self {
            hash: hasher.finish(),
            faces,
        }
    }
}

#[derive(Clone)]
pub(super) struct WordSpan {
    pub(super) start: u32,
    pub(super) end: u32,
    pub(super) face_slot: u16,
    pub(super) run: ShapedRun,
    pub(super) advance: f32,
}

pub(in crate::text::engine) struct WordEntry {
    word: Box<str>,
    level: u8,
    hash: u64,
    faces: SmallVec<[u64; 8]>,
    style: FaceStyle,
    spans: SmallVec<[WordSpan; 1]>,
}

impl WordEntry {
    fn matches(&self, args: &PushRunArgs<'_>, span_key: &SpanKey, style: &FaceStyle) -> bool {
        self.hash == span_key.hash
            && self.level == args.level
            && self.word.as_ref() == args.fragment
            && self.style == *style
            && self.faces.as_slice() == span_key.faces.as_slice()
    }
}

thread_local! {
    static WORDS: RefCell<WordCache> = RefCell::new(Bounded::default());
}

pub(in crate::text::engine) fn with_words<R>(f: impl FnOnce(&mut WordCache) -> R) -> R {
    WORDS.with_borrow_mut(f)
}

pub(super) fn replay(
    words: &WordCache,
    key: u64,
    args: &PushRunArgs<'_>,
    (span_key, style): (&SpanKey, &FaceStyle),
    out: &mut Vec<ShapedSegment>,
) -> bool {
    let entry = words
        .get(&key)
        .filter(|entry| entry.matches(args, span_key, style));
    for span in entry.iter().flat_map(|entry| &entry.spans) {
        out.push(segment(args, span.clone()));
    }
    entry.is_some()
}

pub(super) fn remember(
    words: &mut WordCache,
    key: u64,
    args: &PushRunArgs<'_>,
    (span_key, style): (&SpanKey, &FaceStyle),
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
    words.insert(
        key,
        WordEntry {
            word: args.fragment.into(),
            level: args.level,
            hash: span_key.hash,
            faces: span_key.faces.clone(),
            style: style.clone(),
            spans,
        },
    );
}

pub(super) fn word_key(args: &PushRunArgs<'_>, span_hash: u64) -> u64 {
    let mut hasher = crate::style::text_identity::hasher().build_hasher();
    span_hash.hash(&mut hasher);
    args.fragment.hash(&mut hasher);
    args.level.hash(&mut hasher);
    hasher.finish()
}
