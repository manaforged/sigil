#![cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::expect_used)]

use sigil::prelude::*;
use sigil_kittest::Harness;

#[test]
fn a_view_renders_to_an_image() {
    let Some(image) = Harness::new(|| {
        container()
            .bg(Token::Accent9)
            .layout(LayoutStyle::block().width(40.0).height(30.0))
    })
    .pixels(40, 30)
    .image()
    .expect("image") else {
        return;
    };
    assert_eq!(image.dimensions(), (40, 30));
    let [r, g, b, a] = image.get_pixel(20, 15).0;
    assert!(a == 255 && !(r == g && g == b), "pixel was {r},{g},{b},{a}");
}
