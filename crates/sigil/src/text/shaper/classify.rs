#![allow(clippy::redundant_pub_crate)]

#[cfg(not(feature = "shaping-complex"))]
use std::sync::Once;

use unicode_script::{Script as UScript, UnicodeScript};

use crate::text::shaper::TextSegment;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Script {
    Latin,
    Cjk,
    Common,
    Arabic,
    Hebrew,
    Devanagari,
    Khmer,
    Thai,
    Lao,
    Other,
}

pub(crate) fn classify_segment_script(segment: &TextSegment<'_>) -> Script {
    for ch in segment.text.chars() {
        match ch.script() {
            UScript::Common | UScript::Inherited | UScript::Unknown => {}
            other => return Script::from(other),
        }
    }
    Script::Common
}

impl From<UScript> for Script {
    fn from(script: UScript) -> Self {
        match script {
            UScript::Latin | UScript::Greek | UScript::Cyrillic => Self::Latin,
            UScript::Han
            | UScript::Hiragana
            | UScript::Katakana
            | UScript::Hangul
            | UScript::Bopomofo => Self::Cjk,
            UScript::Arabic => Self::Arabic,
            UScript::Hebrew => Self::Hebrew,
            UScript::Devanagari => Self::Devanagari,
            UScript::Khmer => Self::Khmer,
            UScript::Thai => Self::Thai,
            UScript::Lao => Self::Lao,
            _ => Self::Other,
        }
    }
}

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
