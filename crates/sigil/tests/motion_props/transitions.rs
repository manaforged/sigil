use std::time::Duration;

use sigil::animation::Easing;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::scene::Filter;
use sigil::style::StateStyleBuilder;
use sigil_kittest::Harness;

use super::{
    BLACK, DISTANCE, WHITE, amount, blur, brightness, drop_shadow, filters, gradient_square, hue,
    mount, near, near_color, rgba, run, shadow, shadow_color, square, start, stops,
    under_perspective, width_ratio,
};

fn eased(node: NodeBuilder) -> NodeBuilder {
    node.transition()
        .duration(Duration::from_secs(1))
        .ease(Easing::Linear)
}

fn switched(
    node: NodeBuilder,
    change: impl FnOnce(StateStyleBuilder) -> StateStyleBuilder,
) -> (Harness, Signal<bool>) {
    let flag = signal(false);
    let ui = mount(eased(node).paint_when(flag, change));
    (ui, flag)
}

#[test]
fn filter_transitions_start_missing_functions_at_their_initial_values() {
    Owner::scope("props-filter-transition", || {
        let (mut ui, flag) = switched(square(), |state| {
            state
                .blur(8.0)
                .brightness(2.0)
                .hue_rotate(180.0)
                .drop_shadow(shadow(8.0, 4.0, WHITE))
        });
        assert!(filters(&mut ui).is_empty(), "no filter before the change");
        start(&mut ui, flag);
        run(&mut ui, 0.25);
        let list = filters(&mut ui);
        near(
            amount(&list, blur).expect("blur"),
            2.0,
            0.04,
            "blur from 0px",
        );
        near(
            amount(&list, brightness).expect("brightness"),
            1.25,
            0.006,
            "brightness from 1",
        );
        near(
            amount(&list, hue).expect("hue"),
            45.0,
            0.5,
            "hue-rotate from 0deg",
        );
        let drop = drop_shadow(&list);
        near(drop.offset[1], 2.0, 0.04, "drop-shadow y from 0px");
        near(drop.color[3], 0.25, 0.006, "drop-shadow from transparent");
        run(&mut ui, 1.0);
        let done = filters(&mut ui);
        near(
            amount(&done, blur).expect("blur"),
            8.0,
            0.04,
            "blur at the end",
        );
        near(
            amount(&done, hue).expect("hue"),
            180.0,
            0.5,
            "hue-rotate at the end",
        );
    });
}

#[test]
fn a_drop_shadow_color_transition_mixes_in_srgb() {
    Owner::scope("props-drop-color-transition", || {
        let base = square().drop_shadow(shadow(4.0, 2.0, BLACK));
        let (mut ui, flag) = switched(base, |state| state.drop_shadow(shadow(4.0, 2.0, WHITE)));
        start(&mut ui, flag);
        run(&mut ui, 0.25);
        let color = shadow_color(drop_shadow(&filters(&mut ui)));
        near_color(color, rgba(64, 64, 64, 255), "drop-shadow color at 25%");
    });
}

#[test]
fn a_longer_filter_list_with_the_same_start_interpolates_the_added_function() {
    Owner::scope("props-filter-append", || {
        let (mut ui, flag) = switched(square().blur(2.0), |state| state.brightness(2.0));
        start(&mut ui, flag);
        run(&mut ui, 0.5);
        let list = filters(&mut ui);
        near(
            amount(&list, blur).expect("blur"),
            2.0,
            0.04,
            "the shared blur stays",
        );
        near(
            amount(&list, brightness).expect("brightness"),
            1.5,
            0.006,
            "brightness from 1",
        );
    });
}

#[test]
fn a_mismatched_filter_list_changes_discretely() {
    Owner::scope("props-filter-mismatch", || {
        let (mut ui, flag) = switched(square().brightness(0.5), |state| {
            state.blur(4.0).brightness(1.5)
        });
        start(&mut ui, flag);
        run(&mut ui, 0.25);
        let list = filters(&mut ui);
        assert_eq!(
            list,
            vec![Filter::Blur(4.0), Filter::Brightness(1.5)],
            "brightness then blur-brightness has no matching pairs, so it jumps"
        );
    });
}

#[test]
fn a_gradient_stop_color_transition_mixes_in_srgb() {
    Owner::scope("props-stop-transition", || {
        let (mut ui, flag) = switched(gradient_square(), |state| state.gradient_from(WHITE));
        start(&mut ui, flag);
        run(&mut ui, 0.25);
        near_color(
            stops(&mut ui)[0].1,
            rgba(64, 64, 64, 255),
            "the from stop at 25%",
        );
    });
}

#[test]
fn a_gradient_that_gains_a_stop_changes_discretely() {
    Owner::scope("props-stop-count", || {
        let (mut ui, flag) = switched(gradient_square(), |state| {
            state.gradient_from(WHITE).gradient_via(BLACK)
        });
        start(&mut ui, flag);
        run(&mut ui, 0.25);
        let found = stops(&mut ui);
        assert_eq!(found.len(), 3, "the via stop shows at once");
        near_color(
            found[0].1,
            WHITE,
            "a stop count change does not interpolate",
        );
    });
}

#[test]
fn a_rotate_y_transition_turns_the_node() {
    Owner::scope("props-rotate-y-transition", || {
        let (mut ui, flag) = switched(square(), |state| state.rotate_y(90.0));
        start(&mut ui, flag);
        run(&mut ui, 0.5);
        near(
            width_ratio(&mut ui),
            45_f32.to_radians().cos(),
            2e-3,
            "rotateY(45deg) at 50%",
        );
    });
}

#[test]
fn a_translate_z_transition_moves_the_node_toward_the_viewer() {
    Owner::scope("props-translate-z-transition", || {
        let flag = signal(false);
        let child = eased(square()).paint_when(flag, |state| state.translate_z(200.0));
        let mut ui = mount(under_perspective(child));
        start(&mut ui, flag);
        run(&mut ui, 0.5);
        let want = DISTANCE / (DISTANCE - 100.0);
        near(width_ratio(&mut ui), want, 2e-3, "translateZ(100px) at 50%");
    });
}
