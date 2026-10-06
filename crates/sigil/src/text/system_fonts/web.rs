use std::cell::RefCell;
use std::collections::HashSet;

use crate::text::font_url::load_font_url;
use crate::text::shaper::Script;
use crate::text::web_font::{WebFontScript, web_fallbacks};

thread_local! {
    static REQUESTED: RefCell<HashSet<Script>> = RefCell::new(HashSet::new());
}

const fn web_script(script: Script) -> Option<WebFontScript> {
    match script {
        Script::Cjk => Some(WebFontScript::Cjk),
        Script::Arabic => Some(WebFontScript::Arabic),
        Script::Hebrew => Some(WebFontScript::Hebrew),
        Script::Devanagari => Some(WebFontScript::Devanagari),
        Script::Khmer => Some(WebFontScript::Khmer),
        Script::Thai => Some(WebFontScript::Thai),
        Script::Lao => Some(WebFontScript::Lao),
        Script::Latin | Script::Common | Script::Other => None,
    }
}

pub(super) fn request(script: Script) {
    let Some(key) = web_script(script) else {
        return;
    };
    let first = REQUESTED.with(|requested| requested.borrow_mut().insert(script));
    if !first {
        return;
    }
    for font in web_fallbacks().iter().filter(|font| font.script == key) {
        let _status = load_font_url(&font.url, font.face_index);
    }
}
