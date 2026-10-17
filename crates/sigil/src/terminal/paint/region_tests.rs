use crate::geometry::{Point, Rect};
use crate::scene::command::{CommandScene, DrawRect, SceneCommand, SceneRectStyle};
use crate::style::{Rgba8, Theme};
use crate::terminal::caps::Caps;
use crate::terminal::paint::{Grid, Marks, Paint, raster_regions, raster_with};
use crate::terminal::{ColorDepth, render};

const CELL: f32 = 8.0;

fn scene(rects: &[(f32, f32, f32, f32, Rgba8)]) -> CommandScene {
    let mut scene = CommandScene::default();
    for (x, y, w, h, color) in rects {
        let rect = Rect::new(Point::new(*x, *y), *w, *h);
        scene.commands.push(SceneCommand::Rect(DrawRect::new(
            rect,
            SceneRectStyle::solid(*color),
        )));
    }
    scene
}

fn bytes(grid: &Grid) -> Vec<u8> {
    let mut out = Vec::new();
    render(&mut out, &Grid::new(0, 0), grid, ColorDepth::Truecolor).expect("render");
    out
}

#[test]
fn region_raster_matches_a_full_raster() {
    let theme = Theme::default();
    let marks = Marks::default();
    let paint = Paint {
        theme: &theme,
        cell: CELL,
        caps: Caps::default(),
        marks: &marks,
    };
    let red = Rgba8 {
        r: 220,
        g: 40,
        b: 40,
        a: 255,
    };
    let blue = Rgba8 {
        r: 40,
        g: 40,
        b: 220,
        a: 255,
    };
    let green = Rgba8 {
        r: 40,
        g: 200,
        b: 90,
        a: 255,
    };
    let before = scene(&[
        (0.0, 0.0, 320.0, 96.0, red),
        (16.0, 8.0, 40.0, 4.0, blue),
        (100.0, 50.0, 24.0, 12.0, green),
    ]);
    let after = scene(&[
        (0.0, 0.0, 320.0, 96.0, red),
        (16.0, 8.0, 40.0, 4.0, blue),
        (180.0, 20.0, 24.0, 12.0, green),
    ]);
    let mut retained = Grid::new(40, 12);
    raster_with(&mut retained, &before, &paint);
    let moved = [
        Rect::new(Point::new(96.0, 48.0), 32.0, 16.0),
        Rect::new(Point::new(176.0, 16.0), 32.0, 16.0),
    ];
    let _rows = raster_regions(&mut retained, &after, &paint, &moved);
    let mut fresh = Grid::new(40, 12);
    raster_with(&mut fresh, &after, &paint);
    assert_eq!(
        String::from_utf8_lossy(&bytes(&retained)),
        String::from_utf8_lossy(&bytes(&fresh))
    );
}
