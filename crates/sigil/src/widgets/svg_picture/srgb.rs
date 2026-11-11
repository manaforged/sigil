use std::mem::take;

use crate::geometry::Rect;
use crate::scene::command::{LayerScope, SceneCommand};

use super::AA_MARGIN;

/// Wraps each draw run that lands on pixels its layer already painted in an sRGB layer, since renderers mix a later run with those pixels in linear light where resvg mixes in sRGB.
pub(super) fn separated(commands: Vec<SceneCommand>) -> Vec<SceneCommand> {
    let mut out = Vec::with_capacity(commands.len());
    let mut painted = vec![false];
    let mut run = Vec::new();
    for command in commands {
        if extent(&command).is_some() {
            run.push(command);
            continue;
        }
        close(&mut out, &mut painted, take(&mut run));
        match command {
            SceneCommand::PushLayer(_) => painted.push(false),
            SceneCommand::PopLayer => {
                painted.pop();
                set_painted(&mut painted);
            }
            _ => {}
        }
        out.push(command);
    }
    close(&mut out, &mut painted, run);
    out
}

const fn set_painted(painted: &mut [bool]) {
    if let Some(top) = painted.last_mut() {
        *top = true;
    }
}

fn close(out: &mut Vec<SceneCommand>, painted: &mut [bool], run: Vec<SceneCommand>) {
    if run.is_empty() {
        return;
    }
    let bounds = run
        .iter()
        .filter_map(extent)
        .reduce(Rect::union)
        .filter(|_| painted.last() == Some(&true));
    set_painted(painted);
    let Some(bounds) = bounds else {
        out.extend(run);
        return;
    };
    out.push(SceneCommand::PushLayer(LayerScope::srgb(
        bounds.expand(AA_MARGIN),
    )));
    out.extend(run);
    out.push(SceneCommand::PopLayer);
}

const fn extent(command: &SceneCommand) -> Option<Rect> {
    match command {
        SceneCommand::Path(draw) => Some(draw.bounds),
        SceneCommand::Image(draw) => Some(draw.destination),
        _ => None,
    }
}
