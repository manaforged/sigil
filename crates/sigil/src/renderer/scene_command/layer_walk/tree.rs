use std::borrow::Cow;

use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::{LayerScope, SceneCommand};

pub(super) enum Node<'a> {
    Command(&'a SceneCommand),
    State(Cow<'a, SceneCommand>),
    Layer {
        scope: LayerScope,
        children: Vec<Self>,
    },
    Group {
        opacity: f32,
        commands: &'a [SceneCommand],
        children: Vec<Self>,
    },
}

pub(super) fn build_tree(commands: &[SceneCommand]) -> Vec<Node<'_>> {
    let mut nodes = Vec::new();
    let mut index = 0;
    while let Some(command) = commands.get(index) {
        index += 1;
        match command {
            SceneCommand::PushLayer(scope) => {
                let end = SceneState::layer_end(commands, index);
                push_layer(
                    &mut nodes,
                    *scope,
                    commands.get(index..end).unwrap_or_default(),
                );
                index = end + 1;
            }
            SceneCommand::SaveLayer { opacity } => {
                let end = SceneState::group_end(commands, index);
                let inner = commands.get(index..end).unwrap_or_default();
                if SceneState::composites_group(inner) {
                    nodes.push(Node::Group {
                        opacity: *opacity,
                        commands: inner,
                        children: build_tree(inner),
                    });
                    index = end + 1;
                } else {
                    nodes.push(Node::State(Cow::Borrowed(command)));
                }
            }
            SceneCommand::PopLayer => {}
            other if SceneState::draws(other) => nodes.push(Node::Command(other)),
            other => nodes.push(Node::State(Cow::Borrowed(other))),
        }
    }
    nodes
}

fn push_layer<'a>(nodes: &mut Vec<Node<'a>>, scope: LayerScope, children: &'a [SceneCommand]) {
    let Some(effect) = SceneState::inline_transform(&scope) else {
        nodes.push(Node::Layer {
            scope,
            children: build_tree(children),
        });
        return;
    };
    let state = |command: SceneCommand| Node::State(Cow::Owned(command));
    nodes.push(state(SceneCommand::PushTransform(effect)));
    nodes.push(state(SceneCommand::PushClip(scope.clip)));
    nodes.extend(build_tree(children));
    nodes.push(state(SceneCommand::PopClip));
    nodes.push(state(SceneCommand::PopTransform));
}
