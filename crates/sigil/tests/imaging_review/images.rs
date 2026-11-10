use super::*;
use sigil::widgets::Layout;

fn many_images(count: u32) -> NodeBuilder {
    let mut grid = container().layout(
        LayoutStyle::flex_row()
            .fixed_size(240.0, 40.0)
            .flex_wrap(FlexWrap::Wrap),
    );
    for index in 0..count {
        let color = if index == 0 {
            [255, 0, 0, 255]
        } else {
            [0, (index & 0xff) as u8, (index >> 8) as u8 + 1, 255]
        };
        let source = ImageSource::from_rgba8(1, 1, color.to_vec()).expect("1x1 image");
        grid = grid.child(
            container()
                .layout(LayoutStyle::block().fixed_size(4.0, 4.0))
                .background_image(source),
        );
    }
    page(240.0, 40.0, WHITE, grid)
}

#[test]
fn the_first_of_600_background_images_still_draws() {
    let control = at(
        &shoot(|| many_images(1), 240, 40, Surface::Pixels),
        2.0,
        2.0,
    );
    assert!(
        off(control, RED) <= 40,
        "control, one image draws, got {control:?}"
    );
    let probe = at(
        &shoot(|| many_images(600), 240, 40, Surface::Pixels),
        2.0,
        2.0,
    );
    assert!(
        off(probe, RED) <= 40,
        "the first node's 1x1 red tile, got {probe:?}"
    );
}
