use super::*;
use crate::terminal::paint::Ink;

#[test]
fn image_preserves_cell_backgrounds_and_rejects_empty_sizes() {
    let mut grid = Grid::new(2, 1);
    let red = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    grid.set_row(
        0,
        &[
            Cell {
                bg: Ink::Exact(red),
                ..Cell::blank()
            },
            Cell::blank(),
        ],
    );
    let theme = Theme::default();
    let image = grid
        .image(&theme, PhysicalSize::new(10, 20))
        .expect("cell image");
    assert_eq!(image.get_pixel(5, 10).0, [255, 0, 0, 255]);
    assert_ne!(image.get_pixel(15, 10).0, [255, 0, 0, 255]);
    assert!(matches!(
        grid.image(&theme, PhysicalSize::new(0, 20)),
        Err(error) if error.kind() == crate::ErrorKind::InvalidViewport
    ));
    assert!(matches!(
        Grid::new(0, 0).image(&theme, PhysicalSize::new(10, 20)),
        Err(error) if error.kind() == crate::ErrorKind::InvalidViewport
    ));
}
