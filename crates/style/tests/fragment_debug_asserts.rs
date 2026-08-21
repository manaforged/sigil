#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! DR-015 footgun guards on style APIs for fragments.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use sigil_kernel::{Cx, container, fragment};
use sigil_style::{NodeStyleExt, StyleAtom, Theme};

static ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

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
