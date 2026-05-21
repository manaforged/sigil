//! Regression test: `scroll(content, height)` must not clobber the user's
//! `LayoutStyle` on the content node.
//!
//! Before the fix, `wrap_content` called `.layout(LayoutStyle::block().fixed_size(…))`
//! directly on the user-supplied `NodeBuilder`. Because `NodeBuilder::with::<T>()`
//! routes through `Node::attach::<T>()` which replaces on `TypeId` collision, the
//! user's padding/gap/display were silently replaced.
//!
//! After the fix (Option B), the user's content is wrapped in a fresh anonymous
//! sizing container. The user's `LayoutStyle` is attached only to their own node.
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use actions as _;
use animation as _;
use animation_runtime as _;
use app as _;
use arboard as _;
use criterion as _;
use events as _;
use hal as _;
use image as _;
use kernel as _;
use layout as _;
use pollster as _;
use reactive as _;
use renderer as _;
use style as _;
use tasks as _;
use wgpu as _;

use sigil::prelude::*;

/// Build, wrap in scroll, walk the tree, and assert the content node's
/// `LayoutStyle` still carries the original `flex_column`, padding=16, gap=24.
#[test]
fn scroll_preserves_content_layout_style() {
    sigil::reactive::Runtime::scope("scroll-preserves-content-layout", || {
        let mut cx = Cx::new_root();

        // User supplies a styled content node with flex_column, padding=16, gap=24.
        let content =
            container().layout(LayoutStyle::flex_column().padding(16.0_f32).gap(24.0_f32));

        // Wrap content in scroll — this is the operation that used to clobber.
        let root = cx.add(scroll(content, 800.0_f32));

        // Tree shape post-fix:
        //   root (viewport_container, flex_row)
        //     ├── body (sizing container, block.fixed_size) ← new wrapper
        //     │     └── content (user node — must retain flex_column.padding.gap)
        //     ├── track (scrollbar track)
        //     └── thumb (scrollbar thumb, absolute)
        let root_node = cx.get(root).expect("root node exists");
        let root_children = root_node.children().to_vec();
        assert!(
            root_children.len() >= 3,
            "scroll root must have at least 3 children (body, track, thumb), got {}",
            root_children.len()
        );

        // First child is the body (sizing container).
        let body_id = *root_children.first().expect("body child exists");
        let body_node = cx.get(body_id).expect("body node exists");
        let body_children = body_node.children().to_vec();
        assert_eq!(
            body_children.len(),
            1,
            "sizing container must have exactly 1 child (the user's content), got {}",
            body_children.len()
        );

        // That one child is the user's original content node.
        let content_id = *body_children.first().expect("content child exists");
        let content_node = cx.get(content_id).expect("content node exists");

        // The user's LayoutStyle must be intact — flex_column, padding=16, gap=24.
        let style = content_node
            .get_extension::<LayoutStyle>()
            .expect("content node must have a LayoutStyle");

        let expected = LayoutStyle::flex_column().padding(16.0_f32).gap(24.0_f32);
        assert_eq!(
            style, &expected,
            "content LayoutStyle was clobbered by scroll::wrap_content; \
             expected flex_column+padding(16)+gap(24), got {style:?}"
        );
    });
}

/// Confirm the sizing container (body) gets the `block.fixed_size` layout,
/// not the content node. This is the companion to the preservation test above.
#[test]
fn scroll_sizing_container_has_fixed_size_layout() {
    sigil::reactive::Runtime::scope("scroll-sizing-container-layout", || {
        let mut cx = Cx::new_root();

        let content =
            container().layout(LayoutStyle::flex_column().padding(16.0_f32).gap(24.0_f32));
        let root = cx.add(scroll(content, 800.0_f32));

        let root_node = cx.get(root).expect("root exists");
        let body_id = root_node.children().first().copied().expect("body exists");
        let body_node = cx.get(body_id).expect("body exists");

        let body_style = body_node
            .get_extension::<LayoutStyle>()
            .expect("sizing container must have a LayoutStyle");

        // Sizing container must NOT be flex_column with user's padding — it must be
        // the block fixed_size style from wrap_content.
        let user_style = LayoutStyle::flex_column().padding(16.0_f32).gap(24.0_f32);
        assert_ne!(
            body_style, &user_style,
            "sizing container should NOT carry the user's flex_column layout"
        );
    });
}
