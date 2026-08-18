//! Script classification + complex-shaping warning helpers.
//!
//! Pulled out of `shaper.rs` in G1.7 to keep the file under the 400-line
//! cap. The items here are re-exported via `pub(crate) use` from the
//! parent module, so the crate-internal API surface is unchanged.

#![allow(clippy::redundant_pub_crate)]

#[cfg(not(feature = "shaping-complex"))]
use std::sync::Once;

use unicode_script::{Script as UScript, UnicodeScript};

use crate::shaper::TextSegment;

/// Coarse script classification for routing between the owned
/// shaper and the optional `HarfRust` path (G1.6).
///
/// Latin / CJK / Common / Emoji shape correctly with the owned
/// skrifa shaper. The variants marked `is_complex` need OpenType
/// `GSUB`/`GPOS` shaping plans that only `HarfBuzz` / ``HarfRust``
/// implement at production quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Script {
    /// Latin / Greek / Cyrillic / etc. — owned shaper handles.
    Latin,
    /// CJK Han / Hiragana / Katakana / Hangul / Bopomofo.
    Cjk,
    /// Inherited / Common / Unknown — owned shaper handles.
    Common,
    /// Arabic — joining behaviour + RTL.
    Arabic,
    /// Hebrew — RTL + niqqud positioning.
    Hebrew,
    /// Devanagari — reordering + ligatures.
    Devanagari,
    /// Khmer — complex reordering.
    Khmer,
    /// Thai — line-break + tone-mark positioning.
    Thai,
    /// Lao — similar to Thai.
    Lao,
    /// Anything else not explicitly enumerated; owned shaper.
    Other,
}

impl Script {
    /// Whether this script needs `HarfRust` to shape correctly.
    #[cfg(test)]
    pub(crate) const fn is_complex(self) -> bool {
        matches!(
            self,
            Self::Arabic | Self::Hebrew | Self::Devanagari | Self::Khmer | Self::Thai | Self::Lao,
        )
    }
}

/// Classify the dominant script of a [`TextSegment`] by inspecting
/// its first non-Common / non-Inherited codepoint, falling back to
/// `Common` if none is found. Segments are already script-runs
/// upstream (UAX #24), so the first decisive codepoint is the run's
/// script.
pub(crate) fn classify_segment_script(segment: &TextSegment<'_>) -> Script {
    for ch in segment.text.chars() {
        match ch.script() {
            UScript::Common | UScript::Inherited | UScript::Unknown => {}
            other => return map_u_script(other),
        }
    }
    Script::Common
}

const fn map_u_script(s: UScript) -> Script {
    match s {
        UScript::Latin | UScript::Greek | UScript::Cyrillic => Script::Latin,
        UScript::Han
        | UScript::Hiragana
        | UScript::Katakana
        | UScript::Hangul
        | UScript::Bopomofo => Script::Cjk,
        UScript::Arabic => Script::Arabic,
        UScript::Hebrew => Script::Hebrew,
        UScript::Devanagari => Script::Devanagari,
        UScript::Khmer => Script::Khmer,
        UScript::Thai => Script::Thai,
        UScript::Lao => Script::Lao,
        _ => Script::Other,
    }
}

/// One-shot warning per script when a complex-script segment hits
/// the shaper without the `shaping-complex` feature enabled.
///
/// `Once` per script keeps the log clean: the warning fires once
/// per process per script even if the application paints thousands
/// of Arabic glyphs. Rendering will be visibly buggy (no joining,
/// no LAM-ALEF ligature, no bidi reorder) — the warning is the
/// developer-facing signal to enable the feature.
#[cfg(not(feature = "shaping-complex"))]
pub(crate) fn warn_complex_without_feature(script: Script) {
    static ARABIC: Once = Once::new();
    static HEBREW: Once = Once::new();
    static DEVANAGARI: Once = Once::new();
    static KHMER: Once = Once::new();
    static THAI: Once = Once::new();
    static LAO: Once = Once::new();
    let once = match script {
        Script::Arabic => &ARABIC,
        Script::Hebrew => &HEBREW,
        Script::Devanagari => &DEVANAGARI,
        Script::Khmer => &KHMER,
        Script::Thai => &THAI,
        Script::Lao => &LAO,
        _ => return,
    };
    once.call_once(|| {
        tracing::warn!(
            ?script,
            "complex-script segment without shaping-complex feature; rendering will be visibly buggy",
        );
    });
}
