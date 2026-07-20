//! DR-015 footgun guards on style APIs for fragments.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use allocation_counter as _;
use criterion as _;
use phf as _;
use sigil as _;
use sigil_hal as _;
use sigil_reactive as _;
use sigil_style_macros as _;
use smallvec as _;
use strsim as _;
use tracing as _;

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
