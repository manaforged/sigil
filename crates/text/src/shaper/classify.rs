#![allow(clippy::redundant_pub_crate)]

#[cfg(not(feature = "shaping-complex"))]
use std::sync::Once;

use unicode_script::{Script as UScript, UnicodeScript};

use crate::shaper::TextSegment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
