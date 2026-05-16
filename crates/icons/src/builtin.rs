//! Built-in showcase icon SVG sources.

/// Heart icon SVG source.
pub const HEART_SVG: &str = include_str!("../assets/heart.svg");
/// Gear / settings icon SVG source.
pub const GEAR_SVG: &str = include_str!("../assets/gear.svg");
/// Search / magnifier icon SVG source.
pub const SEARCH_SVG: &str = include_str!("../assets/search.svg");
/// Arrow-right icon SVG source.
pub const ARROW_RIGHT_SVG: &str = include_str!("../assets/arrow-right.svg");
/// Close / X icon SVG source.
pub const X_SVG: &str = include_str!("../assets/x.svg");

/// All built-in icon SVGs in stable showcase order.
pub const ALL: [(&str, &str); 5] = [
    ("heart", HEART_SVG),
    ("gear", GEAR_SVG),
    ("search", SEARCH_SVG),
    ("arrow-right", ARROW_RIGHT_SVG),
    ("x", X_SVG),
];
