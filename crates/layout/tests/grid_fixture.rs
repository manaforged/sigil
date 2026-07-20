#![allow(unused_crate_dependencies)]
//! Grid requests reject loudly (5/5 Wave 3, D-U4).
//!
//! The pre-0.1.x silent `StyleDisplay::Grid` → `BlockSolver` fallback is
//! removed: any node carrying `StyleDisplay::Grid` fails compute with
//! [`LayoutError::GridUnsupported`], at the root or anywhere in the
//! tree. A real grid solver is a later program; until then a loud
//! error beats a silent layout lie.

#![allow(missing_docs)]
#![allow(clippy::float_cmp, clippy::unwrap_used)]

use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_reactive as _;
use slotmap as _;
use smallvec as _;
use taffy as _;

mod stub;

use sigil_hal as _;
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;
use sigil_layout::{Dimension, LayoutError, StyleDisplay};
use sigil_style as _;

use stub::stub_engine;

fn grid_sized(w: f32, h: f32) -> LayoutStyle {
    LayoutStyle::default().with_layout(|style| {
        style.display = StyleDisplay::Grid;
        style.size.width = Dimension::Px(w);
        style.size.height = Dimension::Px(h);
    })
}

/// A grid root rejects with `GridUnsupported` — no silent block fallback.
#[test]
fn grid_root_rejects_with_grid_unsupported() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(grid_sized(200.0, 200.0))
            .child(container().layout(LayoutStyle::block().fixed_size(100.0, 40.0))),
    );
    let err = stub_engine()
        .compute(&mut cx, root, Size::fixed(200.0, 200.0))
        .expect_err("grid root must reject loudly");
    assert!(
        matches!(err, LayoutError::GridUnsupported(_)),
        "expected GridUnsupported, got {err:?}"
    );
}

/// A grid DESCENDANT rejects too — the check lives in per-node sync, not
/// the root dispatch, so it fires anywhere in the tree.
#[test]
fn nested_grid_rejects_with_grid_unsupported() {
    let mut cx = Cx::new_root();
    let root =
        cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(400.0, 400.0))
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(100.0, 40.0))
                        .child(container().layout(grid_sized(80.0, 60.0)).child(
                            container().layout(LayoutStyle::block().fixed_size(20.0, 20.0)),
                        )),
                ),
        );
    let err = stub_engine()
        .compute(&mut cx, root, Size::fixed(400.0, 400.0))
        .expect_err("nested grid must reject loudly");
    assert!(
        matches!(err, LayoutError::GridUnsupported(_)),
        "expected GridUnsupported, got {err:?}"
    );
}
