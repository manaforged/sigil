#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::doc_markdown, clippy::too_many_lines, clippy::print_stdout)]
#![allow(clippy::float_cmp, clippy::missing_const_for_fn)]

use std::rc::Rc;

use sigil::prelude::*;
use sigil::widgets::{FCT_POOL_CAPACITY, FctNode, FctState};
use sigil_kernel::{Cx, NodeId};
use sigil_reactive::{Runtime, signal};
use sigil_style::scale::ScaleRole;
use sigil_style::{Rgba8, Theme};

fn placeholder_color() -> Rgba8 {
    Rgba8 {
        r: 10,
        g: 10,
        b: 10,
        a: 255,
    }
}

#[test]
fn anchored_floater_tracks_position_signal() {
    Runtime::scope("floater-anchor-tracks", || {
        let time = signal(0.0_f32);
        let anchor = signal((100.0_f32, 200.0_f32));
        let state = FctState::new_anchored(1.0, [0.0, -40.0], time, FCT_POOL_CAPACITY, anchor);
        assert!(state.spawn(FctSpawn::new([0.0, 0.0], "42", placeholder_color()), 0.0));

        state.tick(0.25);
        let first = state.snapshot();
        assert_eq!(first.len(), 1);
        assert!((first[0].position[0] - 100.0).abs() < 0.001);
        assert!((first[0].position[1] - 190.0).abs() < 0.001);

        anchor.set((150.0, 220.0));
        state.tick(0.5);
        let second = state.snapshot();
        assert!((second[0].position[0] - 150.0).abs() < 0.001);
        assert!((second[0].position[1] - 200.0).abs() < 0.001);

        assert_ne!(
            first[0].position, second[0].position,
            "moving the anchor signal must move the live particle on the next tick"
        );
    });
}

#[test]
fn floater_rises_and_fades_over_ticks() {
    Runtime::scope("floater-rise-fade", || {
        let time = signal(0.0_f32);
        let state = FctState::new(1.0, [0.0, -60.0], time, FCT_POOL_CAPACITY);
        assert!(state.spawn(FctSpawn::new([40.0, 40.0], "18", placeholder_color()), 0.0));

        let start = state.snapshot();
        assert_eq!(start.len(), 1);
        let start_y = start[0].position[1];
        let start_alpha = start[0].alpha;

        for tick_index in 1..=5_u32 {
            let now = f32::from(tick_index as u16) * 0.1;
            state.tick(now);
        }

        let end = state.snapshot();
        assert_eq!(end.len(), 1);
        assert!(
            end[0].position[1] < start_y,
            "particle must rise (y decreases): start={start_y} end={}",
            end[0].position[1]
        );
        assert!(
            end[0].alpha < start_alpha,
            "particle must fade (alpha decreases): start={start_alpha} end={}",
            end[0].alpha
        );
    });
}

#[test]
fn floater_style_maps_to_theme_tokens() {
    Runtime::scope("floater-style-tokens", || {
        let time = signal(0.0_f32);
        let mut cx = Cx::new_root();

        let damage_spawn = signal(FctSpawn::default());
        let damage_id = cx.add(
            floating_text(damage_spawn)
                .time(time)
                .style(FloaterStyle::Damage),
        );
        let damage_state = fct_state(&cx, damage_id);
        damage_spawn.set(FctSpawn::new([0.0, 0.0], "42", placeholder_color()));
        let damage_color = damage_state.snapshot()[0].color;

        let heal_spawn = signal(FctSpawn::default());
        let heal_id = cx.add(
            floating_text(heal_spawn)
                .time(time)
                .style(FloaterStyle::Heal),
        );
        let heal_state = fct_state(&cx, heal_id);
        heal_spawn.set(FctSpawn::new([0.0, 0.0], "+18", placeholder_color()));
        let heal_color = heal_state.snapshot()[0].color;

        let expected_error = Theme::default().scale(ScaleRole::Error).step(11);
        let expected_success = Theme::default().scale(ScaleRole::Success).step(11);

        assert_eq!(
            damage_color, expected_error,
            "Damage style must resolve the error scale token (step 11)"
        );
        assert_eq!(
            heal_color, expected_success,
            "Heal style must resolve the success scale token (step 11)"
        );
        assert_ne!(
            damage_color,
            placeholder_color(),
            "resolved style color must override the spawn's own color"
        );
        assert_ne!(
            damage_color, heal_color,
            "Damage and Heal must resolve to distinct colors"
        );
    });
}

fn fct_state(cx: &Cx, id: NodeId) -> Rc<FctState> {
    cx.get(id)
        .and_then(|node| node.get_extension::<FctNode>())
        .map(|ext| Rc::clone(ext.state()))
        .expect("floating_text node carries an FctNode extension")
}
