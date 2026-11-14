use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use sigil::Host;
use sigil::animation::{
    Animator, Easing, MotionPrefs, Property, Spring, Transition, every_frame, tick,
};
use sigil::app::{FrameHooks, HostMode};
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::scene::SceneCommand;

const DT_60HZ: f32 = 1.0 / 60.0;
const BAR_HEIGHT: f32 = 20.0;

fn tick_until_settled(animator: &Animator, max_frames: u32) -> u32 {
    let mut frames = 0;
    while animator.active().peek() && frames < max_frames {
        tick(DT_60HZ);
        frames += 1;
    }
    frames
}

fn bar_width_and_opacity(host: &mut Host, bar_color: Color) -> (f32, f32) {
    let scene = host.scene((320.0, 160.0)).expect("pixel scene");
    let mut opacities = vec![1.0_f32];
    let mut found = None;
    for cmd in scene.commands() {
        let opacity = opacities.last().copied().unwrap_or(1.0);
        match cmd {
            SceneCommand::PushLayer(scope) => opacities.push(opacity * scope.alpha()),
            SceneCommand::SaveLayer { opacity: layer } => opacities.push(opacity * layer),
            SceneCommand::PopLayer | SceneCommand::RestoreLayer => {
                if opacities.len() > 1 {
                    opacities.pop();
                }
            }
            SceneCommand::Rect(rect)
                if rect.color() == bar_color && (rect.rect().height - BAR_HEIGHT).abs() < 0.5 =>
            {
                let eff = opacity * rect.alpha();
                found = Some((rect.rect().width, eff));
                break;
            }
            _ => {}
        }
    }
    assert!(found.is_some(), "animated bar rect not found in scene");
    found.unwrap_or_default()
}

#[test]
fn spring_animator_settles_within_known_frame_budget() {
    Owner::scope("spring_animator_settles_within_known_frame_budget", || {
        MotionPrefs::set(false);
        let animator = Animator::spring(0.0_f32, Spring::SMOOTH);
        animator.target(1.0);

        let frames = tick_until_settled(&animator, 240);
        assert!(
            !animator.active().peek(),
            "spring did not settle within 240 frames (~4s @ 60Hz)"
        );
        assert!(
            (animator.value().peek() - 1.0).abs() < 0.005,
            "settled value {} did not reach target 1.0 within epsilon",
            animator.value().peek()
        );
        assert!(
            frames < 240,
            "spring took the full budget — preset may be miscalibrated"
        );
    });
}

#[test]
fn tween_animator_lands_at_target_when_duration_elapses() {
    Owner::scope(
        "tween_animator_lands_at_target_when_duration_elapses",
        || {
            MotionPrefs::set(false);
            let transition = Transition::new()
                .duration(Duration::from_millis(200))
                .ease(Easing::CubicInOut);
            let animator = Animator::tween(0.0_f32, transition);
            animator.target(10.0);

            for _ in 0..15 {
                tick(DT_60HZ);
            }

            assert!(!animator.active().peek());
            assert!(
                (animator.value().peek() - 10.0).abs() < f32::EPSILON * 100.0,
                "tween value {} != target 10.0",
                animator.value().peek()
            );
        },
    );
}

#[test]
fn retarget_during_animation_continues_from_current_value() {
    Owner::scope(
        "retarget_during_animation_continues_from_current_value",
        || {
            MotionPrefs::set(false);
            let animator = Animator::spring(0.0_f32, Spring::SMOOTH);
            animator.target(1.0);

            for _ in 0..5 {
                tick(DT_60HZ);
            }
            let mid = animator.value().peek();
            assert!(
                mid > 0.0 && mid < 1.0,
                "expected mid-flight value, got {mid}"
            );

            animator.target(0.5);
            tick(DT_60HZ);
            let after_retarget = animator.value().peek();
            assert!(
                (after_retarget - mid).abs() < 0.4,
                "retarget jumped: was {mid}, after one frame is {after_retarget}"
            );

            let _ = tick_until_settled(&animator, 240);
            assert!((animator.value().peek() - 0.5).abs() < 0.01);
        },
    );
}

#[test]
fn tick_with_invalid_dt_is_a_no_op() {
    Owner::scope("tick_with_invalid_dt_is_a_no_op", || {
        MotionPrefs::set(false);
        let animator = Animator::spring(0.0_f32, Spring::SMOOTH);
        animator.target(1.0);

        let before = animator.value().peek();
        tick(0.0);
        tick(-1.0);
        tick(f32::NAN);
        tick(f32::INFINITY);
        let after = animator.value().peek();
        assert!(
            (before - after).abs() < f32::EPSILON,
            "invalid dt should be a no-op"
        );
    });
}

fn step_pixel_frames(host: &mut Host, count: usize) {
    for _ in 0..count {
        drop(host.frame((320.0, 160.0), 0.016, FrameHooks::default()));
    }
}

#[test]
fn tween_interpolates_width_and_opacity_on_pixel_host() {
    Owner::scope("tween_interpolates_width_and_opacity_on_pixel_host", || {
        MotionPrefs::set(false);
        let bar_color = Color::rgb(42, 130, 218);
        let slot = Rc::new(RefCell::new(None::<Animator>));
        let slot_mount = Rc::clone(&slot);
        let mut host = Host::new(HostMode::Pixel);
        host.mount(move || {
            let anim = Animator::tween(
                0.0,
                Transition::new()
                    .duration(Duration::from_millis(240))
                    .ease(Easing::CubicOut),
            );
            *slot_mount.borrow_mut() = Some(anim);
            container()
                .width(120.0)
                .height(BAR_HEIGHT)
                .children_reactive(move || {
                    [container()
                        .width(Length::Px((anim.get() * 100.0).max(1.0)))
                        .height(BAR_HEIGHT)
                        .background(bar_color)
                        .drive(Property::Opacity, anim)]
                })
        });

        let anim = slot.borrow().expect("mounted animator");
        anim.target(1.0);
        assert!(anim.active().peek());

        step_pixel_frames(&mut host, 5);
        let (mid_width, mid_opacity) = bar_width_and_opacity(&mut host, bar_color);
        assert!(anim.active().peek());
        assert!(mid_width > 5.0 && mid_width < 99.0, "got {mid_width}");
        assert!(
            mid_opacity > 0.05 && mid_opacity < 0.99,
            "got {mid_opacity}"
        );

        step_pixel_frames(&mut host, 16);
        let (end_width, end_opacity) = bar_width_and_opacity(&mut host, bar_color);
        assert!(!anim.active().peek());
        assert!((end_width - 100.0).abs() < 0.5, "got width {end_width}");
        assert!((end_opacity - 1.0).abs() < 0.01, "got {end_opacity}");
    });
}

#[test]
fn spring_bouncy_custom_snap_and_cancel_behave_as_specified() {
    Owner::scope("spring_bouncy_custom_snap_and_cancel", || {
        MotionPrefs::set(false);
        let bouncy = Animator::spring(0.0, Spring::BOUNCY);
        bouncy.target(1.0);
        let _ = tick_until_settled(&bouncy, 360);
        assert!(!bouncy.active().peek());
        assert!((bouncy.value().peek() - 1.0).abs() < 0.01);

        let custom = Animator::spring(0.0, Spring::new(220.0, 18.0).mass(1.2));
        custom.target(1.0);
        for _ in 0..4 {
            tick(DT_60HZ);
        }
        let held = custom.value().peek();
        assert!(held > 0.0 && held < 1.0);
        custom.cancel();
        assert!(!custom.active().peek());
        tick(DT_60HZ);
        assert!((custom.value().peek() - held).abs() < f32::EPSILON);

        custom.target(1.0);
        assert!(custom.active().peek());
        custom.snap(0.75);
        assert!(!custom.active().peek());
        assert!((custom.value().peek() - 0.75).abs() < f32::EPSILON);

        let sub_epsilon = Animator::spring(0.0, Spring::SMOOTH);
        sub_epsilon.target(0.0005);
        assert!(!sub_epsilon.active().peek());
        assert!((sub_epsilon.value().peek() - 0.0005).abs() < f32::EPSILON);
    });
}

#[test]
fn every_frame_retargets_reentrantly_and_stops_on_owner_disposal() {
    Owner::scope("every_frame_reentrancy_and_disposal", || {
        MotionPrefs::set(false);
        let live = signal(true);
        let ticks = signal(0_u32);
        let dts = Rc::new(RefCell::new(Vec::<f32>::new()));
        let dts_cb = Rc::clone(&dts);
        let retargeted_slot = Rc::new(RefCell::new(None::<Animator>));
        let retargeted_mount = Rc::clone(&retargeted_slot);

        let mut host = Host::new(HostMode::Pixel);
        host.mount(move || {
            let follower = Animator::spring(0.0, Spring::SNAPPY);
            *retargeted_mount.borrow_mut() = Some(follower);
            show(
                live,
                move || {
                    let dts_inner = Rc::clone(&dts_cb);
                    every_frame(move |dt| {
                        dts_inner.borrow_mut().push(dt);
                        let count = ticks.peek() + 1;
                        ticks.set(count);
                        if count == 1 {
                            follower.target(1.0);
                        }
                    });
                    text("running")
                },
                || text("stopped"),
            )
        });

        step_pixel_frames(&mut host, 4);
        let follower = retargeted_slot.borrow().expect("follower animator");
        assert_eq!(ticks.peek(), 4);
        assert_eq!(dts.borrow().len(), 4);
        assert!(dts.borrow().iter().all(|&dt| (dt - 0.016).abs() < 1e-6));
        assert!(follower.value().peek() > 0.0);

        live.set(false);
        step_pixel_frames(&mut host, 1);
        let after_dispose = ticks.peek();
        step_pixel_frames(&mut host, 3);
        assert_eq!(ticks.peek(), after_dispose);
    });
}
