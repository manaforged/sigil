#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::app::DefaultTextMeasure;
use sigil::geometry::{LogicalSize, PhysicalSize, Scale, Size};
use sigil::kernel::{Cx, container};
use sigil::layout::{LayoutEngine, LayoutStyle};
use sigil::reactive::Owner;
use sigil::widgets::Layout;

#[test]
fn scale_change_keeps_launch_viewport_logical() {
    Owner::scope("test", || {
        let logical = LogicalSize::new(720.0, 480.0);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container().layout(LayoutStyle::block().fixed_size(logical.width, logical.height)),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));

        layout
            .compute(&mut cx, root, Size::fixed(logical.width, logical.height))
            .unwrap();
        cx.set_scale(2.0);
        let resized_logical = Scale::new(cx.scale()).to_logical(PhysicalSize::new(1440, 960));
        layout
            .compute(
                &mut cx,
                root,
                Size::fixed(resized_logical.width, resized_logical.height),
            )
            .unwrap();

        let rect = layout.result(root).unwrap().rect();
        assert!((cx.scale() - 2.0).abs() < f32::EPSILON);
        assert_eq!(resized_logical, logical);
        assert_eq!(rect.width.to_bits(), 720.0_f32.to_bits());
        assert_eq!(rect.height.to_bits(), 480.0_f32.to_bits());
    });
}
