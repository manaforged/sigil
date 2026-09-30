#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::kernel::Cx;
use sigil::prelude::*;

#[test]
fn scroll_preserves_content_layout_style() {
    sigil::reactive::Runtime::scope("scroll-preserves-content-layout", || {
        let mut cx = Cx::new_root();

        let content =
            container().layout(LayoutStyle::flex_column().padding(16.0_f32).gap(24.0_f32));

        let root = cx.add(scroll(content).extent(800.0_f32));

        let root_node = cx.get(root).expect("root node exists");
        let root_children = root_node.children().to_vec();
        assert!(
            root_children.len() >= 2,
            "scroll root must have at least 2 children (body, track), got {}",
            root_children.len()
        );

        let body_id = *root_children.first().expect("body child exists");
        let body_node = cx.get(body_id).expect("body node exists");
        let layer_id = *body_node.children().first().expect("scroll layer exists");
        let layer_children = cx
            .get(layer_id)
            .expect("layer node exists")
            .children()
            .to_vec();
        assert_eq!(
            layer_children.len(),
            1,
            "scroll layer must have exactly 1 child (the user's content), got {}",
            layer_children.len()
        );

        let content_id = *layer_children.first().expect("content child exists");
        let content_node = cx.get(content_id).expect("content node exists");

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

#[test]
fn scroll_sizing_container_has_fixed_size_layout() {
    sigil::reactive::Runtime::scope("scroll-sizing-container-layout", || {
        let mut cx = Cx::new_root();

        let content =
            container().layout(LayoutStyle::flex_column().padding(16.0_f32).gap(24.0_f32));
        let root = cx.add(scroll(content).extent(800.0_f32));

        let root_node = cx.get(root).expect("root exists");
        let body_id = root_node.children().first().copied().expect("body exists");
        let body_node = cx.get(body_id).expect("body exists");

        let body_style = body_node
            .get_extension::<LayoutStyle>()
            .expect("sizing container must have a LayoutStyle");

        let user_style = LayoutStyle::flex_column().padding(16.0_f32).gap(24.0_f32);
        assert_ne!(
            body_style, &user_style,
            "sizing container should NOT carry the user's flex_column layout"
        );
    });
}
