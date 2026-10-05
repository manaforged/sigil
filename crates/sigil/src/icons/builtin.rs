use crate::icons::Icon;

pub const HEART_SVG: &str = Icon::Heart.svg();
pub const GEAR_SVG: &str = Icon::Settings.svg();
pub const SEARCH_SVG: &str = Icon::Search.svg();
pub const ARROW_RIGHT_SVG: &str = Icon::ArrowRight.svg();
pub const X_SVG: &str = Icon::X.svg();
pub const ARROW_UP_SVG: &str = Icon::ArrowUp.svg();
pub const ARROW_DOWN_SVG: &str = Icon::ArrowDown.svg();
pub const CHEVRONS_UP_DOWN_SVG: &str = Icon::ChevronsUpDown.svg();
pub const CHECK_SVG: &str = Icon::Check.svg();

pub const ALL: [(&str, &str); 9] = [
    ("heart", HEART_SVG),
    ("gear", GEAR_SVG),
    ("search", SEARCH_SVG),
    ("arrow-right", ARROW_RIGHT_SVG),
    ("x", X_SVG),
    ("arrow-up", ARROW_UP_SVG),
    ("arrow-down", ARROW_DOWN_SVG),
    ("chevrons-up-down", CHEVRONS_UP_DOWN_SVG),
    ("check", CHECK_SVG),
];
