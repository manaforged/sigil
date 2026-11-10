#[cfg(feature = "scene")]
use crate::icons::Icon;
use crate::widgets::{IconBuilder, icon};

macro_rules! glyphs {
    ($(($name:literal, $text:literal $(, $bundled:ident)?)),* $(,)?) => {
        const NAMES: &[(&str, &str)] = &[$(($name, $text)),*];
        #[cfg(feature = "scene")]
        const BUNDLED: &[Option<Icon>] = &[$(glyphs!(@icon $($bundled)?)),*];
    };
    (@icon) => { None };
    (@icon $bundled:ident) => { Some(Icon::$bundled) };
}

glyphs![
    ("accountCircle", "◉", User),
    ("add", "+", Plus),
    ("arrowBack", "←", ArrowLeft),
    ("arrowForward", "→", ArrowRight),
    ("attachFile", "⎘", Link),
    ("calendarToday", "▦", Calendar),
    ("call", "☎"),
    ("camera", "◘"),
    ("check", "✓", Check),
    ("close", "✕", CircleX),
    ("delete", "⌫", Trash2),
    ("download", "↓", Download),
    ("edit", "✎", Pencil),
    ("event", "▦", Calendar),
    ("error", "!", CircleAlert),
    ("fastForward", "»"),
    ("favorite", "♥", Heart),
    ("favoriteOff", "♡"),
    ("folder", "▭", Folder),
    ("help", "?"),
    ("home", "⌂", House),
    ("info", "i", Info),
    ("locationOn", "⌖"),
    ("lock", "▣", Lock),
    ("lockOpen", "□", LockOpen),
    ("mail", "✉", Mail),
    ("menu", "≡", Menu),
    ("moreVert", "⋮", EllipsisVertical),
    ("moreHoriz", "⋯", Ellipsis),
    ("notificationsOff", "◌"),
    ("notifications", "◔", Bell),
    ("pause", "‖", Pause),
    ("payment", "▭"),
    ("person", "◉", User),
    ("phone", "☎"),
    ("photo", "▨"),
    ("play", "▶", Play),
    ("print", "⎙"),
    ("refresh", "↻", RefreshCw),
    ("rewind", "«"),
    ("search", "⌕", Search),
    ("send", "➤"),
    ("settings", "⚙", Settings),
    ("share", "↗", ExternalLink),
    ("shoppingCart", "▿"),
    ("skipNext", "⏭", SkipForward),
    ("skipPrevious", "⏮", SkipBack),
    ("star", "★", Star),
    ("starHalf", "⯪"),
    ("starOff", "☆"),
    ("stop", "■", Square),
    ("upload", "↑", Upload),
    ("visibility", "◎", Eye),
    ("visibilityOff", "◌", EyeOff),
    ("volumeDown", "◁"),
    ("volumeMute", "◁", VolumeX),
    ("volumeOff", "◁", VolumeX),
    ("volumeUp", "◀", Volume2),
    ("warning", "⚠", TriangleAlert),
];

pub(super) fn find(name: &str) -> Option<usize> {
    NAMES.iter().position(|(known, _)| *known == name)
}

pub(super) fn draw(index: Option<usize>) -> IconBuilder {
    let Some(index) = index else {
        return icon("•");
    };
    #[cfg(feature = "scene")]
    if let Some(Some(bundled)) = BUNDLED.get(index) {
        return icon(*bundled);
    }
    icon(NAMES.get(index).map_or("•", |(_, text)| *text))
}
