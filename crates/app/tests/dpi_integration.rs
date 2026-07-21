//! No-window DPI integration coverage for launch-sized viewports.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use accesskit as _;
use crossbeam_channel as _;
use notify as _;
use sigil_a11y as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_renderer as _;
use sigil_shell as _;
use sigil_style as _;
use sigil_tasks as _;
use sigil_text as _;
use toml as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use sigil_app::{AppConfig, DefaultTextMeasure};
use sigil_hal::{LogicalSize, PhysicalSize, Scale};
use sigil_kernel::{Cx, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutEngine, LayoutStyle, Size};
use sigil_reactive::Runtime;

#[test]
fn scale_change_keeps_launch_viewport_logical() {
    Runtime::scope("test", || {
        let config = AppConfig::new("test").size(720, 480);
        assert_eq!(config.size, (720, 480));
        let logical = LogicalSize::new(720.0, 480.0);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container().layout(LayoutStyle::block().fixed_size(logical.width, logical.height)),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));

        layout
            .compute(&mut cx, root, Size::fixed(logical.width, logical.height))
            .unwrap();
        cx.set_scale(Scale::new(2.0));
        let resized_logical = cx.scale().to_logical(PhysicalSize::new(1440, 960));
        layout
            .compute(
                &mut cx,
                root,
                Size::fixed(resized_logical.width, resized_logical.height),
            )
            .unwrap();

        let rect = layout.result(root).unwrap().rect();
        assert_eq!(cx.scale(), Scale::new(2.0));
        assert_eq!(resized_logical, logical);
        assert_eq!(rect.width.to_bits(), 720.0_f32.to_bits());
        assert_eq!(rect.height.to_bits(), 480.0_f32.to_bits());
    });
}
