use crate::shaper::Script;

#[cfg(target_os = "macos")]
pub(crate) const fn candidates(script: Script) -> &'static [&'static str] {
    match script {
        Script::Cjk => &["PingFang SC", "Hiragino Sans", "Apple SD Gothic Neo"],
        Script::Arabic => &["Geeza Pro"],
        Script::Hebrew => &["Arial Hebrew"],
        Script::Devanagari => &["Kohinoor Devanagari", "Devanagari Sangam MN"],
        Script::Khmer => &["Khmer Sangam MN"],
        Script::Thai => &["Thonburi"],
        Script::Lao => &["Lao Sangam MN"],
        Script::Latin | Script::Common | Script::Other => &[],
    }
}

#[cfg(target_family = "windows")]
pub(crate) const fn candidates(script: Script) -> &'static [&'static str] {
    match script {
        Script::Cjk => &["Microsoft YaHei UI", "Yu Gothic UI", "Malgun Gothic"],
        Script::Arabic | Script::Hebrew => &["Segoe UI"],
        Script::Devanagari => &["Nirmala UI"],
        Script::Khmer | Script::Thai | Script::Lao => &["Leelawadee UI"],
        Script::Latin | Script::Common | Script::Other => &[],
    }
}

#[cfg(not(any(target_os = "macos", target_family = "windows")))]
pub(crate) const fn candidates(script: Script) -> &'static [&'static str] {
    match script {
        Script::Cjk => &[
            "Noto Sans CJK SC",
            "Noto Sans CJK JP",
            "Noto Sans CJK KR",
            "Source Han Sans",
            "WenQuanYi Zen Hei",
        ],
        Script::Arabic => &["Noto Sans Arabic", "Noto Naskh Arabic", "DejaVu Sans"],
        Script::Hebrew => &["Noto Sans Hebrew", "DejaVu Sans"],
        Script::Devanagari => &["Noto Sans Devanagari", "Lohit Devanagari"],
        Script::Khmer => &["Noto Sans Khmer"],
        Script::Thai => &["Noto Sans Thai", "Garuda"],
        Script::Lao => &["Noto Sans Lao"],
        Script::Latin | Script::Common | Script::Other => &[],
    }
}
