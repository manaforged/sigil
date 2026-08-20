//! W5.1b — engine reads animator-driven signals at style apply.
//!
//! Verification gate for the binding-install → `AttachedAnimators` →
//! `StyleEngine` path. Drives the same `emit_scene` call the renderer
//! makes and asserts the opacity carried in the emitted scene command
//! reflects the animator's current driven value, not the literal atom.

#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::items_after_statements
)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_tasks as _;
#[cfg(feature = "tui")]
use sigil_tui as _;
use sigil_widgets as _;
use wgpu as _;

use sigil_animation_runtime::{AttachedAnimators, MotionPrefs, install_animators, presets, tick};
use sigil_hal::Size;
use sigil_kernel::{Cx, IntoNode, NodeId, TextContent, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_layout::{LayoutEngine, TextMeasure};
use sigil_reactive::Runtime;
use sigil_renderer::{SceneCommand, emit_scene};
use sigil_style::{NodeStyleExt, StateSlot, StateStyle, StyleAtom, StyleBinding};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(640.0, 480.0))
        .expect("layout");
}

const DT_60HZ: f32 = 1.0 / 60.0;
const OPACITY_BASE: u16 = 5_000;
const OPACITY_HOVER: u16 = 10_000;

/// Find the first scene command's opacity value (cx + root construct a
/// single-rect tree).
fn first_rect_opacity(cx: &Cx, root: NodeId) -> u16 {
    let scene = emit_scene(cx, root);
    for cmd in &scene.commands {
        if let SceneCommand::Rect(rect) = cmd {
            return rect.opacity;
        }
    }
    panic!("no rect command emitted — the scene tree did not produce a background paint");
}

/// Build a single coloured container with a Transition atom and a hover
/// overlay. Returns the root `NodeId`; the caller installs the
/// `AttachedAnimators` overlay onto the binding before emitting.
fn build_animated_node(cx: &mut Cx) -> NodeId {
    static BASE: &[StyleAtom] = &[
        StyleAtom::BackgroundThemeRef("surface"),
        StyleAtom::Opacity(OPACITY_BASE),
        StyleAtom::Transition(presets::DEFAULT),
    ];
    container()
        .class(BASE)
        .layout(LayoutStyle::block().fixed_size(64.0, 32.0))
        .build(cx)
}

/// Install the binding-install hook's output onto the node's
/// `StyleBinding` so the renderer-side `apply_with_overlay` consults it.
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
        compute(&mut cx, root);

        // Frame 0 — no overlay attached yet. Engine reads literal.
        let frame0 = first_rect_opacity(&cx, root);
        assert_eq!(
            frame0, OPACITY_BASE,
            "frame 0 (no overlay yet) reads literal Opacity atom",
        );

        // Build the W5.1a hook output.
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

        // Default → Hover: the animator starts moving toward 10_000.
        attached.retarget_for_state(StateSlot::Hovered, &atoms);
        install_overlay(&mut cx, root, attached);

        // Frame just after install — animator hasn't ticked yet so the
        // signal still reads the base value, but the engine path is
        // now overlay-consulted (test invariant: the SAME value but a
        // different code path).
        let frame_install = first_rect_opacity(&cx, root);
        assert_eq!(
            frame_install, OPACITY_BASE,
            "overlay reads animator signal which equals base before any tick",
        );

        // Tick ten frames at 60 Hz. DEFAULT tween is 200ms ≈ 12 frames,
        // so 10 ticks should leave us strictly between base and target.
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

        // Tick to settled (cap at 2 s of frames for safety).
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
                StyleAtom::BackgroundThemeRef("surface"),
                StyleAtom::Opacity(OPACITY_BASE),
            ];
            let root = container()
                .class(NO_TRANSITION)
                .layout(LayoutStyle::block().fixed_size(64.0, 32.0))
                .build(&mut cx);
            compute(&mut cx, root);

            let opacity = first_rect_opacity(&cx, root);
            assert_eq!(
                opacity, OPACITY_BASE,
                "binding with no overlay produces literal opacity (zero-overhead path)",
            );
        },
    );
}
