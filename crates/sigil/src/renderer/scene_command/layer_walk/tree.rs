use crate::renderer::scene_command::{LayerScope, SceneCommand};

pub(super) enum Node<'a> {
    Command(&'a SceneCommand),
    Scope {
        command: &'a SceneCommand,
        children: Vec<Self>,
    },
    Layer {
        scope: LayerScope,
        children: Vec<Self>,
    },
}

pub(super) fn build_tree(commands: &[SceneCommand]) -> Vec<Node<'_>> {
    let mut index = 0;
    build_until_pop(commands, &mut index)
}

fn build_until_pop<'a>(commands: &'a [SceneCommand], index: &mut usize) -> Vec<Node<'a>> {
    let mut nodes = Vec::new();
    while let Some(command) = commands.get(*index) {
        *index += 1;
        match command {
            SceneCommand::PushLayer(scope) => {
                let children = build_until_pop(commands, index);
                nodes.push(Node::Layer {
                    scope: *scope,
                    children,
                });
            }
            SceneCommand::PushClip(_)
            | SceneCommand::PushTransform(_)
            | SceneCommand::SaveLayer { .. } => {
                let children = build_until_pop(commands, index);
                nodes.push(Node::Scope { command, children });
            }
            SceneCommand::PopLayer
            | SceneCommand::PopClip
            | SceneCommand::PopTransform
            | SceneCommand::RestoreLayer => break,
            other => nodes.push(Node::Command(other)),
        }
    }
    nodes
}

pub(super) fn layer_depth(nodes: &[Node<'_>]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            Node::Command(_) => 0,
            Node::Layer { children, .. } => 1 + layer_depth(children),
            Node::Scope { children, .. } => layer_depth(children),
        })
        .max()
        .unwrap_or(0)
}
