use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::geometry::Rect;
use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::LayoutStyle;
use crate::scene::command::SceneCommand;
use crate::scene::emit_cache::shift_command;
use crate::style::Theme;
use crate::widgets::{CanvasPaint, Layout};

use super::Picture;
use super::lower::lower;

type Lowering = RefCell<Option<(u64, [u32; 2], Arc<[SceneCommand]>)>>;

/// A node that paints `picture` as scene commands, lowered again when the theme or size changes.
pub(super) fn node(picture: Picture, (width, height): (f32, f32)) -> NodeBuilder {
    let last = Lowering::default();
    let paint = CanvasPaint::themed(Rc::new(move |scene, rect, theme| {
        let Some(commands) = lowered(&picture, &last, rect, theme) else {
            return;
        };
        let (dx, dy) = (rect.origin.x, rect.origin.y);
        scene.extend_commands(commands.iter().map(|command| {
            let mut placed = command.clone();
            shift_command(&mut placed, dx, dy);
            placed
        }));
    }));
    container()
        .role(Role::Image)
        .layout(LayoutStyle::block().fixed_size(width, height).shrink(0.0))
        .mutate(move |node| {
            let _ = node.attach(paint);
        })
}

fn lowered(
    picture: &Picture,
    last: &Lowering,
    rect: Rect,
    theme: &Theme,
) -> Option<Arc<[SceneCommand]>> {
    let (key, tree) = picture.tree(theme)?;
    let at = [rect.width.to_bits(), rect.height.to_bits()];
    if let Some((seen, cached, commands)) = last.borrow().as_ref()
        && *seen == key
        && *cached == at
    {
        return Some(Arc::clone(commands));
    }
    let size = (rect.width, rect.height);
    let lowered = lower(&tree, size, key);
    for reason in &lowered.fallbacks {
        tracing::debug!(%reason, "svg subtree drawn as a bitmap");
    }
    let commands: Arc<[SceneCommand]> = lowered.commands.into();
    *last.borrow_mut() = Some((key, at, Arc::clone(&commands)));
    Some(commands)
}
