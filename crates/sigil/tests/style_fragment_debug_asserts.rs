#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::kernel::{Cx, container, fragment};
use sigil::reactive::{Runtime, signal};
use sigil::style::{NodeStyleExt, Paint, Theme, Token};

#[test]
fn paint_on_container_is_allowed() {
    let mut cx = Cx::new_root();
    let _ = cx.add(container().bg(Token::Accent));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "class() on a Fragment")]
fn paint_on_fragment_panics_in_debug() {
    let mut cx = Cx::new_root();
    let _ = cx.add(fragment().bg(Token::Accent));
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
fn paint_when_on_fragment_panics_in_debug() {
    Runtime::scope("paint_when_on_fragment", || {
        let mut cx = Cx::new_root();
        let _ = cx.add(fragment().paint_when(signal(true), |s| s.opacity(0.5)));
    });
}
