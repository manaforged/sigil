use std::cell::RefCell;
use std::hash::{BuildHasher as _, Hash as _, Hasher as _};

use ahash::AHashMap;
use smallvec::SmallVec;

use crate::text::shaper::{FaceStyle, Font};

use super::{PushRunArgs, ShapedSegment, WordSpan, segment, shape_face_spans, u32_clamp};

const WORD_MAX_BYTES: usize = 64;
const WORD_ENTRIES: usize = 4_096;

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
    static WORDS: RefCell<AHashMap<u64, WordEntry>> = RefCell::new(AHashMap::new());
}

pub(in crate::text::engine) fn push_run(
    args: &PushRunArgs<'_>,
    chain: &[Font],
    style: &FaceStyle,
    out: &mut Vec<ShapedSegment>,
) {
    if args.fragment.is_empty() {
        return;
    }
    if args.fragment.len() > WORD_MAX_BYTES {
        shape_face_spans(args, chain, style, out);
        return;
    }
    let key = word_key(args, chain, style);
    if replay(key, args, chain, style, out) {
        return;
    }
    let first = out.len();
    shape_face_spans(args, chain, style, out);
    remember(
        key,
        args,
        chain,
        style,
        out.get(first..).unwrap_or_default(),
    );
}

fn replay(
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

fn remember(
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
        if words.len() >= WORD_ENTRIES {
            words.clear();
        }
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

fn word_key(args: &PushRunArgs<'_>, chain: &[Font], style: &FaceStyle) -> u64 {
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
