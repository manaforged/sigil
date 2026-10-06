use std::cell::RefCell;
use std::collections::HashSet;

use crate::text::font_url::load_font_url;
use crate::text::shaper::Script;

struct WebFontSource {
    script: Script,
    url: &'static str,
    face_index: u32,
}

const SOURCES: &[WebFontSource] = &[
    WebFontSource {
        script: Script::Cjk,
        url: "https://cdn.jsdelivr.net/gh/notofonts/noto-cjk@main/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf",
        face_index: 0,
    },
    WebFontSource {
        script: Script::Arabic,
        url: "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io@main/fonts/NotoSansArabic/hinted/ttf/NotoSansArabic-Regular.ttf",
        face_index: 0,
    },
    WebFontSource {
        script: Script::Hebrew,
        url: "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io@main/fonts/NotoSansHebrew/hinted/ttf/NotoSansHebrew-Regular.ttf",
        face_index: 0,
    },
    WebFontSource {
        script: Script::Devanagari,
        url: "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io@main/fonts/NotoSansDevanagari/hinted/ttf/NotoSansDevanagari-Regular.ttf",
        face_index: 0,
    },
    WebFontSource {
        script: Script::Khmer,
        url: "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io@main/fonts/NotoSansKhmer/hinted/ttf/NotoSansKhmer-Regular.ttf",
        face_index: 0,
    },
    WebFontSource {
        script: Script::Thai,
        url: "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io@main/fonts/NotoSansThai/hinted/ttf/NotoSansThai-Regular.ttf",
        face_index: 0,
    },
    WebFontSource {
        script: Script::Lao,
        url: "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io@main/fonts/NotoSansLao/hinted/ttf/NotoSansLao-Regular.ttf",
        face_index: 0,
    },
];

thread_local! {
    static REQUESTED: RefCell<HashSet<Script>> = RefCell::new(HashSet::new());
}

pub(super) fn request(script: Script) {
    let first = REQUESTED.with(|requested| requested.borrow_mut().insert(script));
    if !first {
        return;
    }
    for source in SOURCES.iter().filter(|source| source.script == script) {
        let _status = load_font_url(source.url, source.face_index);
    }
}
