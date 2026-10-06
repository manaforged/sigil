#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::geometry::Size;
use sigil::kernel::Cx;
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil_kittest::FixedMeasure;

#[test]
fn a_fitted_path_keeps_its_aspect_ratio() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().width(960.0).height(220.0))
            .child(
                path(960, 220)
                    .move_to(0.0, 0.0)
                    .line_to(960.0, 220.0)
                    .close()
                    .fill(Token::Accent9)
                    .fit(),
            ),
    );
    let mut layout = LayoutEngine::with_text_measure(Box::new(FixedMeasure {
        width: 10.0,
        height: 16.0,
    }));
    layout
        .compute(&mut cx, root, Size::fixed(960.0, 220.0))
        .unwrap();
    let child = cx
        .get(root)
        .and_then(|node| node.children().first().copied())
        .unwrap();
    let rect = layout.result(child).unwrap().rect();
    assert_eq!((rect.width, rect.height), (960.0, 220.0));
}
