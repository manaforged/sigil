#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil_kittest::Harness;

#[test]
fn nowrap_text_shrunk_by_flex_paints_on_one_line() {
    Runtime::scope("text-nowrap-shrunk", || {
        let mut ui = Harness::new(|| {
            column().child(
                container()
                    .layout(LayoutStyle::flex_row().width(220.0).height(20.0).gap(6.0))
                    .child(text("8e34241").nowrap())
                    .child(
                        text("guard word graph against lane overflow")
                            .nowrap()
                            .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 0.0)),
                    )
                    .child(text("sam").nowrap()),
            )
        })
        .pixels(400, 120);
        ui.settle();
        let image = ui.image().unwrap().expect("gpu");
        let backdrop = image.get_pixel(image.width() - 1, image.height() - 1).0;
        let below = (24..image.height())
            .flat_map(|y| (0..image.width()).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let px = image.get_pixel(x, y).0;
                px.iter().zip(backdrop).any(|(a, b)| a.abs_diff(b) > 12)
            })
            .count();
        assert_eq!(
            below, 0,
            "nowrap text drew {below} pixels below its one-line row"
        );
    });
}
