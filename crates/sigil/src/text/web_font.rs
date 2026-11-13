use std::sync::{PoisonError, RwLock};

/// The script a web fallback font covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WebFontScript {
    /// Chinese, Japanese, and Korean text; the default font is Noto Sans CJK SC.
    Cjk,
    /// Arabic script; the default font is Noto Sans Arabic.
    Arabic,
    /// Hebrew script; the default font is Noto Sans Hebrew.
    Hebrew,
    /// Devanagari script; the default font is Noto Sans Devanagari.
    Devanagari,
    /// Khmer script; the default font is Noto Sans Khmer.
    Khmer,
    /// Thai script; the default font is Noto Sans Thai.
    Thai,
    /// Lao script; the default font is Noto Sans Lao.
    Lao,
}

/// A fallback font that the web build downloads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebFont {
    /// The script whose first use in shaped text starts the download.
    pub(crate) script: WebFontScript,
    /// The address the font file is fetched from.
    pub(crate) url: String,
    /// The 0-based index of the face to load from the font file.
    pub(crate) face_index: u32,
}

impl WebFont {
    /// Creates a web font for `script` from face `face_index` of the file at `url`.
    #[must_use]
    pub fn new(script: WebFontScript, url: impl Into<String>, face_index: u32) -> Self {
        Self {
            script,
            url: url.into(),
            face_index,
        }
    }

    /// The address the font file is fetched from.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the default Noto fallback fonts.
    #[must_use]
    pub fn noto_defaults() -> Vec<Self> {
        NOTO_DEFAULTS
            .iter()
            .map(|(script, url)| Self::new(*script, *url, 0))
            .collect()
    }
}

const NOTO_DEFAULTS: &[(WebFontScript, &str)] = &[
    (
        WebFontScript::Cjk,
        "https://cdn.jsdelivr.net/gh/notofonts/noto-cjk@f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf",
    ),
    (
        WebFontScript::Arabic,
        "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io@51a53382c7fca5180dd6030d79a4464c1a2730c8/fonts/NotoSansArabic/hinted/ttf/NotoSansArabic-Regular.ttf",
    ),
    (
        WebFontScript::Hebrew,
        "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io@51a53382c7fca5180dd6030d79a4464c1a2730c8/fonts/NotoSansHebrew/hinted/ttf/NotoSansHebrew-Regular.ttf",
    ),
    (
        WebFontScript::Devanagari,
        "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io@51a53382c7fca5180dd6030d79a4464c1a2730c8/fonts/NotoSansDevanagari/hinted/ttf/NotoSansDevanagari-Regular.ttf",
    ),
    (
        WebFontScript::Khmer,
        "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io@51a53382c7fca5180dd6030d79a4464c1a2730c8/fonts/NotoSansKhmer/hinted/ttf/NotoSansKhmer-Regular.ttf",
    ),
    (
        WebFontScript::Thai,
        "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io@51a53382c7fca5180dd6030d79a4464c1a2730c8/fonts/NotoSansThai/hinted/ttf/NotoSansThai-Regular.ttf",
    ),
    (
        WebFontScript::Lao,
        "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io@51a53382c7fca5180dd6030d79a4464c1a2730c8/fonts/NotoSansLao/hinted/ttf/NotoSansLao-Regular.ttf",
    ),
];

static WEB_FALLBACKS: RwLock<Option<Vec<WebFont>>> = RwLock::new(None);

/// Replaces the web fallback fonts.
#[doc(hidden)]
pub fn set_web_fallbacks(fonts: impl IntoIterator<Item = WebFont>) {
    let fonts = fonts.into_iter().collect();
    *WEB_FALLBACKS
        .write()
        .unwrap_or_else(PoisonError::into_inner) = Some(fonts);
}

/// The installed web fallback fonts; the Noto defaults when none are installed.
#[doc(hidden)]
#[must_use]
#[cfg(any(target_arch = "wasm32", feature = "kittest"))]
pub fn web_fallbacks() -> Vec<WebFont> {
    WEB_FALLBACKS
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
        .unwrap_or_else(WebFont::noto_defaults)
}
