    use super::*;

    /// A spring with a non-zero displacement and reasonable damping
    /// must settle on its target in a bounded number of frames.
    #[test]
    fn smooth_spring_settles_within_two_seconds() {
        let mut spring = Spring::from_preset(0.0, SpringPreset::Smooth);
        spring.set_target(1.0);
        let mut frames = 0;
        while !spring.is_settled() && frames < 240 {
            spring.step(1.0 / 60.0);
            frames += 1;
        }
        assert!(
            spring.is_settled(),
            "Smooth spring did not settle in 2 s (frames={frames}, value={value})",
            value = spring.value()
        );
        assert!((spring.value() - 1.0).abs() < Spring::DEFAULT_VALUE_EPSILON);
    }

    /// A bouncy spring should overshoot at least once before settling
    /// — that's the whole point.
    #[test]
    fn bouncy_spring_overshoots() {
        let mut spring = Spring::from_preset(0.0, SpringPreset::Bouncy);
        spring.set_target(1.0);
        let mut max = f32::NEG_INFINITY;
        for _ in 0..240 {
            spring.step(1.0 / 60.0);
            if spring.value() > max {
                max = spring.value();
            }
        }
        assert!(max > 1.0, "Bouncy spring did not overshoot 1.0 (max={max})");
    }

    /// A stiff spring with a small delta should settle almost
    /// immediately.
    #[test]
    fn stiff_spring_settles_fast() {
        let mut spring = Spring::from_preset(0.0, SpringPreset::Stiff);
        spring.set_target(0.1);
        let mut frames = 0;
        while !spring.is_settled() && frames < 60 {
            spring.step(1.0 / 60.0);
            frames += 1;
        }
        assert!(frames < 30, "Stiff spring took {frames} frames (>0.5 s)");
    }

    /// A long `dt` (frame hitch, debugger break) must NOT diverge a
    /// stiff spring — the substep clamp keeps it inside the stable
    /// region of semi-implicit Euler. Without the clamp, a 200 ms step
    /// against `Stiff` (k=500) sends `value` to ±1e9 within a few
    /// calls.
    #[test]
    fn stiff_spring_does_not_diverge_on_long_dt() {
        let mut spring = Spring::from_preset(0.0, SpringPreset::Stiff);
        spring.set_target(1.0);
        // Simulate ten 200 ms hitches in a row.
        for _ in 0..10 {
            spring.step(0.2);
            assert!(
                spring.value().is_finite() && spring.value().abs() < 10.0,
                "Spring diverged on long dt: value={value}, velocity={vel}",
                value = spring.value(),
                vel = spring.velocity(),
            );
        }
    }

    /// Non-finite `dt` (`NaN`, `±inf`) or non-positive `dt` is a no-op,
    /// not a panic.
    #[test]
    fn step_rejects_non_finite_or_non_positive_dt() {
        let mut spring = Spring::from_preset(0.0, SpringPreset::Smooth);
        spring.set_target(1.0);
        let before = (spring.value(), spring.velocity());
        spring.step(f32::NAN);
        assert_eq!((spring.value(), spring.velocity()), before);
        spring.step(f32::INFINITY);
        assert_eq!((spring.value(), spring.velocity()), before);
        spring.step(-1.0);
        assert_eq!((spring.value(), spring.velocity()), before);
        spring.step(0.0);
        assert_eq!((spring.value(), spring.velocity()), before);
    }

    /// Reset wipes everything — value, velocity, target.
    #[test]
    fn reset_clears_motion() {
        let mut spring = Spring::from_preset(0.0, SpringPreset::Smooth);
        spring.set_target(1.0);
        for _ in 0..5 {
            spring.step(1.0 / 60.0);
        }
        assert!(spring.velocity().abs() > 0.0);
        spring.reset(5.0);
        assert!((spring.value() - 5.0).abs() < f32::EPSILON);
        assert!(spring.velocity().abs() < f32::EPSILON);
        assert!((spring.target() - 5.0).abs() < f32::EPSILON);
        assert!(spring.is_settled());
    }

    /// `step` is deterministic — same inputs, same outputs, every time.
    /// This is load-bearing for replay determinism and golden frames.
    #[test]
    fn step_is_deterministic() {
        let run = || {
            let mut spring = Spring::from_preset(0.0, SpringPreset::Smooth);
            spring.set_target(1.0);
            let mut samples = Vec::new();
            for _ in 0..30 {
                spring.step(1.0 / 60.0);
                samples.push(spring.value());
            }
            samples
        };
        let a = run();
        let b = run();
        assert_eq!(a.len(), b.len());
        for (sa, sb) in a.iter().zip(&b) {
            assert!((sa - sb).abs() < f32::EPSILON);
        }
    }

    /// Zero mass would divide-by-zero inside step; the constructor
    /// must reject it.
    #[test]
    #[should_panic(expected = "Spring mass must be positive")]
    fn zero_mass_panics() {
        let _ = Spring::new(0.0, 100.0, 10.0, 0.0);
    }

    /// Pixel-space animations want looser epsilons than the
    /// progress-signal defaults. A spring with 0.5 epsilons settles
    /// sooner than one with default 0.001 epsilons under the same
    /// physics, proving the threshold is wired into `is_settled`.
    #[test]
    fn with_epsilons_changes_settle_time() {
        let tight = {
            let mut s = Spring::from_preset(0.0, SpringPreset::Smooth);
            s.set_target(1.0);
            s
        };
        let loose = {
            let mut s = Spring::from_preset(0.0, SpringPreset::Smooth).with_epsilons(0.05, 0.05);
            s.set_target(1.0);
            s
        };

        let frames_until_settled = |mut spring: Spring| -> u32 {
            let mut frames = 0;
            while !spring.is_settled() && frames < 600 {
                spring.step(1.0 / 60.0);
                frames += 1;
            }
            frames
        };

        let tight_frames = frames_until_settled(tight);
        let loose_frames = frames_until_settled(loose);
        assert!(
            loose_frames < tight_frames,
            "Looser epsilons should settle sooner \
             (tight={tight_frames}, loose={loose_frames})"
        );
    }

    /// A non-positive epsilon would prevent settling forever; the
    /// builder must reject it.
    #[test]
    #[should_panic(expected = "Spring epsilons must be positive")]
    fn zero_epsilon_panics() {
        let _ = Spring::from_preset(0.0, SpringPreset::Smooth).with_epsilons(0.0, 0.001);
    }
