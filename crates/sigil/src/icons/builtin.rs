use crate::icons::Icon;

#[cfg(any(test, feature = "desktop", feature = "kittest", feature = "terminal"))]
/// The heart icon.
pub(crate) const HEART_SVG: &str = Icon::Heart.svg();
#[cfg(any(test, feature = "desktop", feature = "kittest", feature = "terminal"))]
/// The gear icon.
pub(crate) const GEAR_SVG: &str = Icon::Settings.svg();
#[cfg(any(test, feature = "desktop", feature = "kittest", feature = "terminal"))]
/// The search icon.
pub(crate) const SEARCH_SVG: &str = Icon::Search.svg();
#[cfg(any(test, feature = "desktop", feature = "kittest", feature = "terminal"))]
/// The right arrow icon.
pub(crate) const ARROW_RIGHT_SVG: &str = Icon::ArrowRight.svg();
#[cfg(any(test, feature = "desktop", feature = "kittest", feature = "terminal"))]
/// The X icon.
pub(crate) const X_SVG: &str = Icon::X.svg();
/// The up arrow icon.
pub(crate) const ARROW_UP_SVG: &str = Icon::ArrowUp.svg();
/// The down arrow icon.
pub(crate) const ARROW_DOWN_SVG: &str = Icon::ArrowDown.svg();
/// The up and down chevrons icon.
pub(crate) const CHEVRONS_UP_DOWN_SVG: &str = Icon::ChevronsUpDown.svg();
#[cfg(any(test, feature = "desktop", feature = "kittest", feature = "terminal"))]
/// The check mark icon.
pub(crate) const CHECK_SVG: &str = Icon::Check.svg();

#[cfg(any(test, feature = "desktop", feature = "kittest", feature = "terminal"))]
/// Lists every bundled icon as a `(name, svg)` pair.
pub(crate) const ALL: [(&str, &str); 9] = [
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
