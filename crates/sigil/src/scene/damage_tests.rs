use super::{Changes, Damage, cell_damage};
use crate::geometry::{Point, Rect};
use crate::scene::command::{CommandScene, DrawRect, SceneCommand, SceneRectStyle};
use crate::style::Rgba8;

const CELL: (f32, f32) = (8.0, 8.0);
const RED: Rgba8 = Rgba8 {
    r: 220,
    g: 40,
    b: 40,
    a: 255,
};
const BLUE: Rgba8 = Rgba8 {
    r: 40,
    g: 40,
    b: 220,
    a: 255,
};

fn area() -> Rect {
    Rect::new(Point::new(0.0, 0.0), 320.0, 96.0)
}

fn scene(rects: &[(f32, f32, Rgba8)]) -> CommandScene {
    let mut scene = CommandScene::default();
    for (x, y, color) in rects {
        let rect = Rect::new(Point::new(*x, *y), 16.0, 8.0);
        scene.commands.push(SceneCommand::Rect(DrawRect::new(
            rect,
            SceneRectStyle::solid(*color),
        )));
    }
    scene
}

fn regions(before: &CommandScene, after: &CommandScene) -> Damage<Vec<Rect>> {
    cell_damage((before, after), Changes::Diff, area(), CELL)
}

#[test]
fn a_one_command_change_damages_exactly_its_cells() {
    let before = scene(&[(0.0, 0.0, RED), (80.0, 40.0, RED)]);
    let after = scene(&[(0.0, 0.0, RED), (80.0, 40.0, BLUE)]);
    match regions(&before, &after) {
        Damage::Regions(found) => {
            assert_eq!(found, vec![Rect::new(Point::new(80.0, 40.0), 16.0, 8.0)]);
        }
        other => panic!("expected one region, got {other:?}"),
    }
}

#[test]
fn two_distant_changes_give_two_regions() {
    let before = scene(&[(0.0, 0.0, RED), (240.0, 80.0, RED)]);
    let after = scene(&[(0.0, 0.0, BLUE), (240.0, 80.0, BLUE)]);
    match regions(&before, &after) {
        Damage::Regions(found) => assert_eq!(found.len(), 2, "{found:?}"),
        other => panic!("expected two regions, got {other:?}"),
    }
}

#[test]
fn an_unchanged_scene_has_no_damage() {
    let before = scene(&[(0.0, 0.0, RED), (80.0, 40.0, BLUE)]);
    assert!(matches!(
        regions(&before, &before.clone()),
        Damage::Unchanged
    ));
}
