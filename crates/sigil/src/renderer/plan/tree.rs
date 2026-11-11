use std::borrow::Cow;

use crate::scene::command::{BlendMode, DrawPath, LayerEffect, LayerScope, Mask, SceneCommand};
use crate::scene::state::SceneState;

#[derive(Clone, Copy)]
pub(crate) enum MaskSource<'a> {
    Path(&'a DrawPath),
    Scene(&'a Mask),
}

pub(crate) enum PlanNode<'a> {
    Command(&'a SceneCommand),
    State(Cow<'a, SceneCommand>),
    Layer {
        scope: LayerScope,
        mask: Option<MaskSource<'a>>,
        children: Vec<Self>,
        commands: &'a [SceneCommand],
    },
    Group {
        opacity: f32,
        commands: &'a [SceneCommand],
        children: Vec<Self>,
    },
}

pub(crate) fn build_tree(commands: &[SceneCommand]) -> Vec<PlanNode<'_>> {
    let mut nodes = Vec::new();
    let mut index = 0;
    while let Some(command) = commands.get(index) {
        index += 1;
        match command {
            SceneCommand::PushLayer(scope) => {
                let end = SceneState::layer_end(commands, index);
                push_layer(
                    &mut nodes,
                    scope,
                    commands.get(index..end).unwrap_or_default(),
                );
                index = end + 1;
            }
            SceneCommand::PushClip(clip) if clip.composites() => {
                let end = SceneState::clip_end(commands, index);
                let inner = commands.get(index..end).unwrap_or_default();
                nodes.push(PlanNode::Layer {
                    scope: clip.scope(),
                    mask: clip.mask().map(MaskSource::Path),
                    children: build_tree(inner),
                    commands: inner,
                });
                index = end + 1;
            }
            #[cfg(host)]
            SceneCommand::SaveLayer { opacity } => {
                let end = SceneState::group_end(commands, index);
                let inner = commands.get(index..end).unwrap_or_default();
                if SceneState::composites_group(inner) {
                    nodes.push(PlanNode::Group {
                        opacity: *opacity,
                        commands: inner,
                        children: build_tree(inner),
                    });
                    index = end + 1;
                } else {
                    nodes.push(PlanNode::State(Cow::Borrowed(command)));
                }
            }
            SceneCommand::PopLayer => {}
            other if SceneState::draws(other) => nodes.push(PlanNode::Command(other)),
            other => nodes.push(PlanNode::State(Cow::Borrowed(other))),
        }
    }
    nodes
}

fn push_layer<'a>(
    nodes: &mut Vec<PlanNode<'a>>,
    scope: &'a LayerScope,
    children: &'a [SceneCommand],
) {
    let inline = SceneState::inline_transform(scope).filter(|_| scope.blend == BlendMode::Normal);
    let Some(effect) = inline else {
        let mask = match &scope.effect {
            LayerEffect::Mask(mask) => Some(MaskSource::Scene(mask)),
            _ => None,
        };
        nodes.push(PlanNode::Layer {
            scope: scope.clone(),
            mask,
            children: build_tree(children),
            commands: children,
        });
        return;
    };
    let state = |command: SceneCommand| PlanNode::State(Cow::Owned(command));
    nodes.push(state(SceneCommand::PushTransform(effect)));
    nodes.push(state(SceneCommand::PushClip(scope.clip.into())));
    nodes.extend(build_tree(children));
    nodes.push(state(SceneCommand::PopClip));
    nodes.push(state(SceneCommand::PopTransform));
}
