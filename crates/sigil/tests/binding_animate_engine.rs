#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::items_after_statements
)]

#[path = "widget_support/mod.rs"]
mod widget_support;

use sigil::animation_runtime::{AttachedAnimators, MotionPrefs, install_animators, presets, tick};
use sigil::hal::Size;
use sigil::kernel::{Cx, IntoNode, NodeId, container};
use sigil::layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil::reactive::Runtime;
use sigil::scene::{SceneCommand, emit_scene};
use sigil::style::{ColorKey, Token};
use sigil::style::{NodeStyleExt, StateSlot, StateStyle, StyleAtom, StyleBinding};
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
    let scene = emit_scene(cx, root);
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
    panic!("no rect command emitted — the scene tree did not produce a background paint");
}

fn build_animated_node(cx: &mut Cx) -> NodeId {
    static BASE: &[StyleAtom] = &[
        StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::Surface)),
        StyleAtom::Opacity(OPACITY_BASE),
        StyleAtom::Transition(presets::DEFAULT),
    ];
    container()
        .class(BASE)
        .layout(LayoutStyle::block().fixed_size(64.0, 32.0))
        .build(cx)
}

fn install_overlay(cx: &mut Cx, root: NodeId, attached: AttachedAnimators) {
    let node = cx.get_mut(root).expect("node");
    let binding = node
        .get_extension_mut::<StyleBinding>()
        .expect("binding installed by .class()");
    binding.set_animator_overlay(Some(Box::new(attached)));
}

#[test]
fn engine_reads_animator_signal_over_ticks() {
    Runtime::scope("w5_1b_engine_reads_animator_signal_over_ticks", || {
        MotionPrefs::set(false);
        let mut cx = Cx::new_root();
        let root = build_animated_node(&mut cx);
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        let frame0 = first_rect_opacity(&cx, root);
        assert_eq!(
            frame0, OPACITY_BASE,
            "frame 0 (no overlay yet) reads literal Opacity atom",
        );

        let atoms = [
            StyleAtom::Opacity(OPACITY_BASE),
            StyleAtom::Transition(presets::DEFAULT),
        ];
        let state_style = StateStyle::EMPTY.with_hovered(&[StyleAtom::Opacity(OPACITY_HOVER)]);
        let mut attached = install_animators(&atoms, state_style, 1);
        assert_eq!(
            attached.len(),
            1,
            "one numeric delta in the hover slot → one animator",
        );

        attached.retarget_for_state(StateSlot::Hovered, &atoms);
        install_overlay(&mut cx, root, attached);

        let frame_install = first_rect_opacity(&cx, root);
        assert_eq!(
            frame_install, OPACITY_BASE,
            "overlay reads animator signal which equals base before any tick",
        );

        for _ in 0..10 {
            tick(DT_60HZ);
        }
        let mid = first_rect_opacity(&cx, root);
        assert!(
            mid > OPACITY_BASE,
            "after 10 ticks opacity must have advanced toward hover \
             (got {mid}, base={OPACITY_BASE})",
        );
        assert!(
            mid < OPACITY_HOVER,
            "after 10 ticks opacity must not yet have reached hover \
             (got {mid}, target={OPACITY_HOVER})",
        );

        for _ in 0..120 {
            tick(DT_60HZ);
        }
        let settled = first_rect_opacity(&cx, root);
        assert_eq!(
            settled, OPACITY_HOVER,
            "after settled the engine reads the animator's final value \
             (the hover target)",
        );
    });
}

#[test]
fn engine_falls_back_to_literal_when_no_animator() {
    Runtime::scope(
        "w5_1b_engine_falls_back_to_literal_when_no_animator",
        || {
            MotionPrefs::set(false);
            let mut cx = Cx::new_root();
            static NO_TRANSITION: &[StyleAtom] = &[
                StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::Surface)),
                StyleAtom::Opacity(OPACITY_BASE),
            ];
            let root = container()
                .class(NO_TRANSITION)
                .layout(LayoutStyle::block().fixed_size(64.0, 32.0))
                .build(&mut cx);
            compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

            let opacity = first_rect_opacity(&cx, root);
            assert_eq!(
                opacity, OPACITY_BASE,
                "binding with no overlay produces literal opacity (zero-overhead path)",
            );
        },
    );
}
