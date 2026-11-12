use crate::geometry::Constraint;
use crate::kernel::{Cx, NodeId, container};
use crate::layout::LayoutEngine;
use crate::reactive::Owner;

use super::SceneCache;

fn laid_out() -> (Cx, NodeId) {
    let mut cx = Cx::new_root();
    let root = cx.add(container());
    LayoutEngine::new(Box::new(crate::layout::FakeMeasure::fixed(0.0, 0.0)))
        .compute(&mut cx, root, Constraint::fixed(100.0, 80.0))
        .expect("layout");
    (cx, root)
}

#[test]
fn replays_only_when_context_root_viewport_and_scale_match() {
    Owner::scope("scene-cache-replay", || {
        let (cx, root) = laid_out();
        let mut cache = SceneCache::default();

        let _ = cache.scene(&cx, (root, (100.0, 80.0)), (false, None));
        let _ = cache.scene(&cx, (root, (100.0, 80.0)), (true, None));
        assert_eq!(cache.emissions, 1, "matching expose redraw replays");

        let _ = cache.scene(&cx, (root, (120.0, 80.0)), (true, None));
        assert_eq!(cache.emissions, 2, "viewport change rebuilds");
    });
}

#[test]
fn an_opaque_window_paints_the_theme_surface_behind_the_tree() {
    Owner::scope("scene-cache-backdrop", || {
        let (cx, root) = laid_out();
        let surface = crate::style::Theme::default().color(crate::style::Token::Surface);

        let opaque = super::window_scene(&cx, root, (100.0, 80.0), true);
        let backdrop = match opaque.commands.first() {
            Some(crate::scene::SceneCommand::Rect(rect)) => Some((rect.color, rect.rect)),
            _ => None,
        };
        let (color, rect) = backdrop.expect("an opaque window starts with the backdrop");
        assert_eq!(color, surface);
        assert!(
            rect.origin.x < 0.0
                && rect.origin.y < 0.0
                && rect.origin.x + rect.width > 100.0
                && rect.origin.y + rect.height > 80.0,
            "the backdrop's edge lies outside the viewport: {rect:?}"
        );

        let clear = super::window_scene(&cx, root, (100.0, 80.0), false);
        assert_eq!(clear.commands.len() + 1, opaque.commands.len());
    });
}

#[test]
fn a_frame_whose_paint_was_skipped_keeps_the_next_scene_current() {
    use crate::app::FrameHooks;
    use crate::app::embed::Host;
    use crate::kernel::HostMode;
    use crate::prelude::*;
    const VIEW: (f32, f32) = (300.0, 200.0);
    Owner::scope("scene-cache-skipped-paint", || {
        let first = signal(String::from("alpha"));
        let second = signal(String::from("beta"));
        let mut host = Host::new(HostMode::Pixel);
        host.mount(move || {
            let mut rows = column()
                .child(text(move || first.get()))
                .child(text(move || second.get()));
            for index in 0..40 {
                rows = rows.child(text(format!("row {index}")));
            }
            rows
        });
        let mut cache = SceneCache::default();
        let frame = |target: &mut Host, memo: &mut SceneCache, paint: bool| {
            let hooks = FrameHooks {
                input: None,
                after_layout: None,
                paint: Some(Box::new(
                    |painted: &mut Host, info: &crate::app::FrameInfo| {
                        if !paint {
                            memo.skip();
                        } else if let Some((cx, _, _, root)) = painted.tree() {
                            let _ = memo.scene(cx, (root, VIEW), (false, Some(&info.delta)));
                        }
                    },
                )),
            };
            let _ = target.frame(VIEW, 0.0, hooks);
        };
        frame(&mut host, &mut cache, true);
        frame(&mut host, &mut cache, true);
        first.set(String::from("omega"));
        frame(&mut host, &mut cache, false);
        second.set(String::from("delta"));
        frame(&mut host, &mut cache, true);
        let (cx, _, _, root) = host.tree().expect("tree");
        let fresh = super::window_scene(cx, root, VIEW, false);
        assert!(
            cache.scene.commands == fresh.commands,
            "cached scene differs from a fresh emit after a frame whose paint was skipped"
        );
    });
}
