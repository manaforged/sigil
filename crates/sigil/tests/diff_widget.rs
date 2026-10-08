#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::cell::Cell;
use std::fmt::Write as _;
use std::rc::Rc;

use sigil::kernel::Role;
use sigil::prelude::*;
use sigil::widgets::{DiffView, Patch, Selection, diff};
use sigil_kittest::{Harness, Query};

fn long_patch(lines: usize) -> String {
    let mut patch = format!("--- a/big.rs\n+++ b/big.rs\n@@ -1,{lines} +1,{lines} @@\n");
    for at in 0..lines {
        let _ = writeln!(patch, "-let value_{at} = {at};");
        let _ = writeln!(patch, "+let value_{at} = {};", at + 1);
    }
    patch
}

#[test]
fn a_5000_line_patch_builds_only_the_visible_rows() {
    let patch = long_patch(2_500);
    let mut ui = Harness::new(move || diff(patch.clone()).view(DiffView::Unified)).size(100, 40);
    ui.settle();
    let labels = ui.find_all(&Query::role(Role::Label)).len();
    assert!(labels < 2_000, "the diff built {labels} labels");
}

#[test]
fn a_click_on_a_hunk_header_selects_its_changed_lines() {
    let slot: Rc<Cell<Option<Signal<Selection>>>> = Rc::new(Cell::new(None));
    let export = Rc::clone(&slot);
    let text = "--- a/x.rs\n+++ b/x.rs\n@@ -1,2 +1,2 @@ fn main\n keep\n-old\n+new\n";
    let patch = Patch::parse(text);
    assert_eq!(patch.hunks.len(), 1);
    assert_eq!(
        patch.hunks.first().and_then(|hunk| hunk.path.as_deref()),
        Some("x.rs")
    );
    let mut ui = Harness::new(move || {
        let selection = signal(Selection::default());
        export.set(Some(selection));
        diff(text.to_owned())
            .view(DiffView::Unified)
            .select(selection)
    })
    .size(80, 12);
    ui.settle();
    let header = ui
        .find_all(&Query::role(Role::Label))
        .into_iter()
        .find(|node| node.value().is_some_and(|value| value.contains("fn main")))
        .expect("hunk header row");
    ui.click(&header);
    ui.settle();
    let picked = slot.get().expect("selection signal").peek();
    assert!(picked.hunks.contains(&0));
    assert_eq!(
        picked.lines.len(),
        2,
        "the hunk has one removed and one added line"
    );
}

fn saturated(image: &image::RgbaImage, area: sigil::geometry::Rect) -> usize {
    let (x0, y0) = (area.origin.x.max(0.0) as u32, area.origin.y.max(0.0) as u32);
    let (x1, y1) = (
        ((area.origin.x + area.width) as u32).min(image.width()),
        ((area.origin.y + area.height) as u32).min(image.height()),
    );
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let [r, g, b, _] = image.get_pixel(x, y).0;
            r.max(g).max(b) - r.min(g).min(b) > 60
        })
        .count()
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn rust_code_in_a_diff_is_syntax_coloured() {
    let text = "--- a/x.rs\n+++ b/x.rs\n@@ -1,2 +1,2 @@\n pub fn keep(value: u32) -> Option<u32> { Some(42) }\n-let a = 1;\n+let a = 2;\n";
    let mut ui = Harness::new(move || {
        column()
            .bg(Token::Surface)
            .child(diff(text.to_owned()).view(DiffView::Unified))
    })
    .pixels(900, 240);
    ui.settle();
    let image = ui.image().unwrap().expect("gpu");
    let line = ui
        .find_all(&Query::role(Role::Label))
        .into_iter()
        .find(|node| {
            node.value()
                .is_some_and(|value| value.contains("pub fn keep"))
        })
        .expect("context line");
    assert!(
        saturated(&image, line.bounds().unwrap()) > 0,
        "the context line is one grey"
    );
}
