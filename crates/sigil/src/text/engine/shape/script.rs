use crate::text::shaper::Script;

pub(in crate::text::engine) fn char_script(ch: char) -> Script {
    use unicode_script::{Script as UScript, UnicodeScript};
    if ch.is_ascii() {
        return if ch.is_ascii_alphabetic() {
            Script::Latin
        } else {
            Script::Common
        };
    }
    match ch.script() {
        UScript::Common | UScript::Inherited | UScript::Unknown => Script::Common,
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

#[cfg(any(test, feature = "kittest"))]
pub(in crate::text::engine) const fn script_name(script: Script) -> &'static str {
    match script {
        Script::Latin => "Latin",
        Script::Cjk => "Cjk",
        Script::Common => "Common",
        Script::Arabic => "Arabic",
        Script::Hebrew => "Hebrew",
        Script::Devanagari => "Devanagari",
        Script::Khmer => "Khmer",
        Script::Thai => "Thai",
        Script::Lao => "Lao",
        Script::Other => "Other",
    }
}
