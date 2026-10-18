use super::{Damage, cell_damage};
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
    cell_damage((before, after), None, area(), CELL)
}

#[test]
fn a_one_command_change_damages_exactly_its_cells() {
    let before = scene(&[(0.0, 0.0, RED), (80.0, 40.0, RED)]);
    let after = scene(&[(0.0, 0.0, RED), (80.0, 40.0, BLUE)]);
    assert_eq!(
        regions(&before, &after),
        Damage::Regions(vec![Rect::new(Point::new(80.0, 40.0), 16.0, 8.0)])
    );
}

#[test]
fn two_distant_changes_give_two_regions() {
    let before = scene(&[(0.0, 0.0, RED), (240.0, 80.0, RED)]);
    let after = scene(&[(0.0, 0.0, BLUE), (240.0, 80.0, BLUE)]);
    let found = regions(&before, &after);
    assert!(
        matches!(&found, Damage::Regions(found) if found.len() == 2),
        "{found:?}"
    );
}

#[test]
fn an_unchanged_scene_has_no_damage() {
    let before = scene(&[(0.0, 0.0, RED), (80.0, 40.0, BLUE)]);
    assert!(matches!(
        regions(&before, &before.clone()),
        Damage::Unchanged
    ));
}

#[test]
fn scattered_changes_fall_back_to_a_full_raster() {
    let steady: Vec<(f32, f32, Rgba8)> = (0..600).map(|_| (1000.0, 0.0, RED)).collect();
    let spots = |color: Rgba8| {
        (0..13_u8)
            .flat_map(|col| {
                (0..6_u8).map(move |row| (f32::from(col) * 24.0, f32::from(row) * 16.0, color))
            })
            .chain(steady.iter().copied())
            .collect::<Vec<_>>()
    };
    let found = regions(&scene(&spots(RED)), &scene(&spots(BLUE)));
    assert!(matches!(found, Damage::Full(_)), "{found:?}");
}

fn spot(index: usize) -> (f32, f32) {
    let index = u16::try_from(index).unwrap_or(u16::MAX);
    (f32::from(index % 10) * 32.0, f32::from(index / 10) * 8.0)
}

fn spots(count: usize, paint: impl Fn(usize) -> Rgba8) -> Vec<(f32, f32, Rgba8)> {
    (0..count)
        .map(|index| {
            let (x, y) = spot(index);
            (x, y, paint(index))
        })
        .collect()
}

fn cell_of(index: usize) -> Rect {
    let (x, y) = spot(index);
    Rect::new(Point::new(x, y), 16.0, 8.0)
}

#[test]
fn an_insert_in_the_middle_damages_only_the_new_command() {
    let before = spots(40, |_| RED);
    let mut after = before.clone();
    let (x, y) = spot(45);
    after.insert(17, (x, y, BLUE));
    assert_eq!(
        regions(&scene(&before), &scene(&after)),
        Damage::Regions(vec![cell_of(45)])
    );
}

#[test]
fn a_removal_in_the_middle_damages_only_the_old_command() {
    let mut before = spots(40, |_| RED);
    let (x, y) = spot(45);
    before.insert(23, (x, y, BLUE));
    let after = spots(40, |_| RED);
    assert_eq!(
        regions(&scene(&before), &scene(&after)),
        Damage::Regions(vec![cell_of(45)])
    );
}

#[test]
fn two_changes_far_apart_damage_only_their_commands() {
    let before = spots(40, |_| RED);
    let after = spots(
        40,
        |index| if index == 3 || index == 36 { BLUE } else { RED },
    );
    let found = regions(&scene(&before), &scene(&after));
    let Damage::Regions(mut found) = found else {
        unreachable!("expected regions, got {found:?}");
    };
    found.sort_by(|a, b| a.origin.y.total_cmp(&b.origin.y));
    assert_eq!(found, vec![cell_of(3), cell_of(36)]);
}

#[test]
fn changing_more_than_half_a_large_scene_repaints_whole() {
    let before = spots(80, |_| RED);
    let half = spots(80, |index| if index % 4 == 0 { BLUE } else { RED });
    assert!(matches!(
        regions(&scene(&before), &scene(&half)),
        Damage::Regions(_)
    ));
    let most = spots(80, |index| {
        if index % 4 == 0 || index == 1 {
            BLUE
        } else {
            RED
        }
    });
    assert_eq!(
        regions(&scene(&before), &scene(&most)),
        Damage::Full(super::FullReason::ChangedShare)
    );
}

fn layered(effect: crate::scene::command::LayerEffect, color: Rgba8) -> CommandScene {
    let mut layer = CommandScene::default();
    layer
        .commands
        .push(SceneCommand::PushLayer(crate::scene::command::LayerScope {
            clip: area(),
            clip_radii: crate::scene::corner_radii::CornerRadii::default(),
            effect,
            opacity: 1.0,
        }));
    layer
        .commands
        .extend(scene(&[(80.0, 40.0, color)]).commands);
    layer.commands.push(SceneCommand::PopLayer);
    layer
}

#[test]
fn only_a_layer_that_moves_cells_off_their_grid_repaints_whole() {
    use crate::scene::command::LayerEffect;
    let shifted = LayerEffect::Transform([[1.0, 0.0, 8.0], [0.0, 1.0, 0.0]]);
    for effect in [LayerEffect::Identity, shifted] {
        let found = regions(&layered(effect, RED), &layered(effect, BLUE));
        assert_ne!(
            found,
            Damage::Full(super::FullReason::NonLocalLayerEffect),
            "{effect:?}"
        );
    }
    let skewed = LayerEffect::Transform([[1.0, 0.5, 0.0], [0.0, 1.0, 0.0]]);
    let leaning = LayerEffect::Transform([[1.0, 0.0, 0.0], [0.5, 1.0, 0.0]]);
    let blurred = LayerEffect::LayerBlur { sigma: 2.0 };
    for effect in [skewed, leaning, blurred] {
        assert_eq!(
            regions(&layered(effect, RED), &layered(effect, BLUE)),
            Damage::Full(super::FullReason::NonLocalLayerEffect),
            "{effect:?}"
        );
    }
}

fn clipped(clip: Rect, inside: Rgba8) -> CommandScene {
    let mut clipped = CommandScene::default();
    clipped.commands.push(SceneCommand::PushClip(clip));
    clipped
        .commands
        .extend(scene(&[(80.0, 40.0, inside)]).commands);
    clipped.commands.push(SceneCommand::PopClip);
    clipped
        .commands
        .extend(scene(&[(240.0, 80.0, RED)]).commands);
    clipped
}

#[test]
fn a_clip_that_shrinks_damages_both_clips() {
    let wide = Rect::new(Point::new(80.0, 40.0), 32.0, 8.0);
    let narrow = Rect::new(Point::new(80.0, 40.0), 16.0, 8.0);
    assert_eq!(
        regions(&clipped(wide, RED), &clipped(narrow, RED)),
        Damage::Regions(vec![wide])
    );
}

#[test]
fn a_change_inside_an_unchanged_clip_damages_only_itself() {
    let clip = Rect::new(Point::new(64.0, 32.0), 64.0, 24.0);
    assert_eq!(
        regions(&clipped(clip, RED), &clipped(clip, BLUE)),
        Damage::Regions(vec![cell_of_point(80.0, 40.0)])
    );
}

fn cell_of_point(x: f32, y: f32) -> Rect {
    Rect::new(Point::new(x, y), 16.0, 8.0)
}

fn group(opacity: f32) -> CommandScene {
    let mut group = CommandScene::default();
    group.commands.push(SceneCommand::SaveLayer { opacity });
    group.commands.extend(scene(&[(80.0, 40.0, RED)]).commands);
    group.commands.push(SceneCommand::RestoreLayer);
    group.commands.extend(scene(&[(240.0, 80.0, RED)]).commands);
    group
}

#[test]
fn a_faded_group_damages_only_its_children() {
    assert_eq!(
        regions(&group(1.0), &group(0.5)),
        Damage::Regions(vec![cell_of_point(80.0, 40.0)])
    );
}

fn moved(offset: f32) -> CommandScene {
    let mut moved = CommandScene::default();
    moved.commands.push(SceneCommand::PushTransform([
        [1.0, 0.0, offset],
        [0.0, 1.0, 0.0],
    ]));
    moved.commands.extend(scene(&[(80.0, 40.0, RED)]).commands);
    moved.commands.push(SceneCommand::PopTransform);
    moved
}

#[test]
fn a_changed_transform_repaints_whole() {
    assert_eq!(
        regions(&moved(0.0), &moved(8.0)),
        Damage::Full(super::FullReason::UnboundedChange)
    );
}
