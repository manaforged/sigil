#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::kernel::{Cx, container, fragment};
use sigil::style::{ColorKey, NodeStyleExt, StyleAtom, Theme, Token};

static ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef(ColorKey::Token(
    Token::Accent,
))];

#[test]
fn class_on_container_is_allowed() {
    let mut cx = Cx::new_root();
    let _ = cx.add(container().class("bg-accent"));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "class() on a Fragment")]
fn class_on_fragment_panics_in_debug() {
    let mut cx = Cx::new_root();
    let _ = cx.add(fragment().class("bg-accent"));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "theme() on a Fragment")]
fn theme_on_fragment_panics_in_debug() {
    let mut cx = Cx::new_root();
    let _ = cx.add(fragment().theme(Theme::default()));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "style_reactive() on a Fragment")]
fn style_reactive_on_fragment_panics_in_debug() {
    let mut cx = Cx::new_root();
    let _ = cx.add(fragment().style_reactive(|| ATOMS));
}
