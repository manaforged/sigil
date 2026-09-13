#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;
use crate::kernel::{Cx, IntoNode};
use crate::reactive::{Runtime, signal};

fn build_strip(chrome: impl FnOnce(TabsBuilder) -> TabsBuilder) {
    Runtime::scope("tabs-chrome", || {
        let active = signal(0usize);
        let strip = chrome(tabs(active, vec!["A".into(), "B".into()]).label("test"));
        let mut cx = Cx::new_root();
        let id = strip.build(&mut cx);
        assert!(cx.get(id).is_some(), "tabs root exists");
    });
}

#[test]
fn line_and_pill_build_on_default_theme() {
    build_strip(super::TabsBuilder::line);
    build_strip(super::TabsBuilder::pill);
}
