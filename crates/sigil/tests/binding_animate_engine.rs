#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "widget_support/mod.rs"]
mod widget_support;

use sigil::animation_runtime::{MotionPrefs, presets, tick};
use sigil::events::{Event, EventRouter};
use sigil::geometry::{Point, Size};
use sigil::kernel::{Cx, NodeId};
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::scene::{SceneCommand, emit_scene};
use widget_support::compute;

const DT_60HZ: f32 = 1.0 / 60.0;
const OPACITY_BASE: u16 = 5_000;
const OPACITY_HOVER: u16 = 10_000;
const VIEWPORT: Size = Size::fixed(640.0, 480.0);

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "effective opacity is clamped to 0..10000"
)]
fn first_rect_opacity(cx: &Cx, root: NodeId) -> u16 {
    let scene = emit_scene(cx, root, None);
    let mut opacities = vec![1.0_f32];
    for cmd in &scene.commands {
        let opacity = opacities.last().copied().expect("root opacity");
        match cmd {
            SceneCommand::PushLayer(scope) => opacities.push(opacity * scope.opacity),
            SceneCommand::SaveLayer { opacity: layer } => opacities.push(opacity * layer),
            SceneCommand::PopLayer | SceneCommand::RestoreLayer => {
                opacities.pop();
            }
            SceneCommand::Rect(rect) => {
                return (f32::from(rect.opacity) * opacity)
                    .round()
                    .clamp(0.0, 10000.0) as u16;
            }
            _ => {}
        }
    }
    panic!("no rect command emitted");
}

#[test]
fn a_style_transition_paints_values_between_base_and_hover_then_settles() {
    Owner::scope("binding-animate-hover", || {
        MotionPrefs::set(false);
        let mut cx = Cx::new_root();
        let root = cx.add(
            interactive(
                container()
                    .bg(Token::Surface)
                    .layout(LayoutStyle::block().fixed_size(100.0, 100.0)),
            )
            .opacity(0.5)
            .transition(presets::DEFAULT)
            .hover(|state| state.opacity(1.0)),
        );
        let mut layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        assert_eq!(first_rect_opacity(&cx, root), OPACITY_BASE, "rest value");

        router.route(
            &mut cx,
            &layout,
            Event::pointer_move(Point::new(50.0, 50.0)),
        );
        layout.compute(&mut cx, root, VIEWPORT).expect("layout");
        assert_eq!(
            first_rect_opacity(&cx, root),
            OPACITY_BASE,
            "a transition starts at the base value"
        );

        for _ in 0..10 {
            tick(DT_60HZ);
        }
        let mid = first_rect_opacity(&cx, root);
        assert!(
            mid > OPACITY_BASE && mid < OPACITY_HOVER,
            "after 10 ticks the value must lie between base and hover, got {mid}"
        );

        for _ in 0..120 {
            tick(DT_60HZ);
        }
        assert_eq!(
            first_rect_opacity(&cx, root),
            OPACITY_HOVER,
            "settled value"
        );
    });
}
