mod tree;

pub(crate) use tree::{PlanNode, build_tree};

use crate::geometry::Rect;
use crate::scene::command::SceneCommand;
use crate::scene::command::effects::transform_rect;
use crate::scene::state::SceneState;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Batch {
    Rect,
    Fill,
    Shadow,
    Decoration,
    Glyphs,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flush {
    Shapes,
    All,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanStep {
    Skip,
    Queue { batch: Batch, flush_glyphs: bool },
    Dispatch(Flush),
}

pub(crate) fn needs_layers(commands: &[SceneCommand]) -> bool {
    commands
        .iter()
        .any(|command| matches!(command, SceneCommand::PushLayer(_)))
        || SceneState::has_composited_group(commands)
}

pub(crate) const fn batch_of(command: &SceneCommand) -> Option<Batch> {
    match command {
        SceneCommand::GlyphRun(_) => Some(Batch::Glyphs),
        SceneCommand::Rect(_) => Some(Batch::Rect),
        SceneCommand::Gradient(_) | SceneCommand::Pattern(_) => Some(Batch::Fill),
        SceneCommand::DrawShadow { .. } => Some(Batch::Shadow),
        SceneCommand::Decoration(_) => Some(Batch::Decoration),
        _ => None,
    }
}

pub(crate) fn step(
    state: &SceneState,
    command: &SceneCommand,
    glyphs_under: impl FnOnce(Rect) -> bool,
) -> PlanStep {
    if state.clipped_out() && SceneState::draws(command) {
        return PlanStep::Skip;
    }
    match (batch_of(command), command) {
        (Some(Batch::Glyphs), _) => PlanStep::Queue {
            batch: Batch::Glyphs,
            flush_glyphs: false,
        },
        (Some(Batch::Rect), SceneCommand::Rect(draw)) => PlanStep::Queue {
            batch: Batch::Rect,
            flush_glyphs: glyphs_under(transform_rect(state.current_transform(), draw.rect)),
        },
        (Some(batch), _) => PlanStep::Queue {
            batch,
            flush_glyphs: true,
        },
        (None, SceneCommand::PushClip(_) | SceneCommand::PopClip) => {
            PlanStep::Dispatch(Flush::Shapes)
        }
        (None, _) => PlanStep::Dispatch(Flush::All),
    }
}
