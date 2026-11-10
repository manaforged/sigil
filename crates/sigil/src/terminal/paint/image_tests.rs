use crate::geometry::{Point, Rect};
use crate::kernel::{ImageSource, ObjectFit};
use crate::scene::command::{CommandScene, DrawImage, SceneCommand};
use crate::style::Theme;
use crate::terminal::caps::Caps;
use crate::terminal::paint::{CellPaint, Grid, Marks, raster_with};

fn placed(cell: Option<[u16; 2]>, fit: ObjectFit) -> (u16, u16) {
    let source = ImageSource::from_rgba8(4, 4, vec![255; 64]).expect("image");
    let mut scene = CommandScene::default();
    scene.commands.push(SceneCommand::Image(DrawImage {
        node: None,
        source,
        fit,
        destination: Rect::new(Point::new(0.0, 0.0), 80.0, 80.0),
        opacity: u16::MAX,
        crop: None,
        raster: None,
    }));
    let (theme, marks) = (Theme::default(), Marks::default());
    let paint = CellPaint {
        theme: &theme,
        cell: 8.0,
        caps: Caps {
            cell,
            ..Caps::default()
        },
        marks: &marks,
    };
    let mut grid = Grid::new(20, 20);
    raster_with(&mut grid, &scene, &paint);
    let place = grid.images().first().expect("placement");
    (place.cols, place.rows)
}

#[test]
fn a_contained_square_image_stays_square_on_tall_cells() {
    assert_eq!(placed(Some([10, 20]), ObjectFit::Contain), (10, 5));
    assert_eq!(placed(None, ObjectFit::Contain), (10, 5));
    assert_eq!(placed(Some([10, 20]), ObjectFit::Fill), (10, 10));
}
