use super::SceneState;
use crate::hal::{Point, Rect};
use crate::renderer::scene_command::SceneCommand;
use crate::renderer::scene_command::walk::IDENTITY;

#[test]
fn a_stray_pop_keeps_the_layer_bounds_clip() {
    let bounds = Rect::new(Point::new(0.0, 0.0), 10.0, 10.0);
    let mut state = SceneState::new(Some(bounds), IDENTITY, 1.0);
    state.apply(&SceneCommand::PushClip(Rect::new(
        Point::new(0.0, 0.0),
        5.0,
        5.0,
    )));
    state.apply(&SceneCommand::PopClip);
    state.apply(&SceneCommand::PopClip);
    assert_eq!(
        state.clip_rect(Rect::new(Point::new(20.0, 20.0), 5.0, 5.0)),
        None
    );
}

#[test]
fn nested_opacity_multiplies_and_restores() {
    let mut state = SceneState::default();
    state.apply(&SceneCommand::SaveLayer { opacity: 0.5 });
    state.apply(&SceneCommand::SaveLayer { opacity: 0.5 });
    assert!((state.current_opacity() - 0.25).abs() < f32::EPSILON);
    state.apply(&SceneCommand::RestoreLayer);
    state.apply(&SceneCommand::RestoreLayer);
    state.apply(&SceneCommand::RestoreLayer);
    assert!((state.current_opacity() - 1.0).abs() < f32::EPSILON);
}
