#![allow(clippy::unwrap_used)]

use std::fmt;
use std::time::{Duration, Instant};

use crate::animation_runtime::{Animator, MotionPrefs, active_count, config, presets};
use crate::events::ShellEvent;
use crate::geometry::{LogicalSize, PhysicalSize, Scale};
use crate::hal::Shell;
use crate::kernel::{Cx, container};
use crate::layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle};
use crate::reactive::{Runtime, signal};
use crate::tasks::{AsyncRuntime, signal_writer};

use crate::Host;
use crate::app::config::AppConfig;
use crate::app::launch::{
    AnimationClock, begin_effect, create_initial_window, layout_for_scale, run_layout_if_dirty,
};
use crate::app::text_measure::DefaultTextMeasure;

#[test]
fn completion_rebuilds_before_accessibility_reads_replaced_controls() {
    Runtime::scope("completion-rebuild", || {
        let revision = signal(0);
        let mut host = Host::cells();
        host.mount(move || {
            container().children_reactive(move || {
                let value = signal(format!("Profile {}", revision.get()));
                vec![container().text_value(value)]
            })
        });
        host.layout((200.0, 100.0)).unwrap();
        let router = crate::events::EventRouter::new();
        let before = host.accessibility_tree(&router);
        assert!(
            before
                .nodes
                .iter()
                .any(|(_, node)| node.value() == Some("Profile 0"))
        );
        let mut async_rt = AsyncRuntime::new().unwrap();
        async_rt.install_current();
        signal_writer(revision).set(1).unwrap();

        begin_effect(&mut async_rt);
        Runtime::begin_layout();
        run_layout_if_dirty(Runtime::frame_dirty(), || {
            host.layout((200.0, 100.0)).unwrap();
        });
        Runtime::begin_paint();
        let after = host.accessibility_tree(&router);
        assert!(
            after
                .nodes
                .iter()
                .any(|(_, node)| node.value() == Some("Profile 1"))
        );
        assert!(
            !after
                .nodes
                .iter()
                .any(|(_, node)| node.value() == Some("Profile 0"))
        );
        Runtime::end_frame();
    });
}

#[derive(Debug)]
struct MockError;

impl fmt::Display for MockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("mock shell error")
    }
}

impl std::error::Error for MockError {}

#[derive(Clone)]
struct MockWaker;

#[derive(Debug, PartialEq)]
enum CreateCall {
    Physical(String, PhysicalSize),
    Logical(String, LogicalSize),
    With(String, LogicalSize, crate::app::WindowOpts),
}

#[derive(Default)]
struct MockShell {
    calls: Vec<CreateCall>,
}

impl Shell for MockShell {
    type Error = MockError;
    type Waker = MockWaker;
    type Window = usize;

    fn waker(&self) -> Self::Waker {
        MockWaker
    }

    fn wake(_waker: &Self::Waker) -> Result<(), Self::Error> {
        Ok(())
    }

    fn create_window(
        &mut self,
        title: &str,
        initial_size: PhysicalSize,
    ) -> Result<Self::Window, Self::Error> {
        self.calls
            .push(CreateCall::Physical(title.to_owned(), initial_size));
        Ok(self.calls.len() - 1)
    }

    fn create_window_logical(
        &mut self,
        title: &str,
        initial: LogicalSize,
    ) -> Result<Self::Window, Self::Error> {
        self.calls
            .push(CreateCall::Logical(title.to_owned(), initial));
        Ok(self.calls.len() - 1)
    }

    fn create_window_with(
        &mut self,
        title: &str,
        initial: LogicalSize,
        opts: crate::app::WindowOpts,
    ) -> Result<Self::Window, Self::Error> {
        self.calls
            .push(CreateCall::With(title.to_owned(), initial, opts));
        Ok(self.calls.len() - 1)
    }

    fn drain_events(&mut self) -> std::vec::IntoIter<ShellEvent> {
        Vec::new().into_iter()
    }

    fn run<F>(self, _frame: F) -> Result<(), Self::Error>
    where
        F: FnMut(&mut Self) + 'static,
    {
        Ok(())
    }
}

#[test]
fn initial_window_passes_logical_size_and_default_opts() {
    let mut shell = MockShell::default();
    let config = AppConfig::new("test").size(720, 480);
    let logical = LogicalSize::new(720.0, 480.0);

    let handle = create_initial_window(&mut shell, &config, logical).unwrap();

    assert_eq!(handle, 0);
    assert_eq!(
        shell.calls,
        vec![CreateCall::With(
            "test".to_owned(),
            logical,
            crate::app::WindowOpts::default()
        )]
    );
}

#[test]
fn native_initial_window_ignores_web_canvas() {
    let logical = LogicalSize::new(720.0, 480.0);
    let mut plain = MockShell::default();
    let mut bound = MockShell::default();

    create_initial_window(&mut plain, &AppConfig::new("test"), logical).unwrap();
    create_initial_window(&mut bound, &AppConfig::new("test").canvas("app"), logical).unwrap();

    assert_eq!(bound.calls, plain.calls);
}

#[test]
fn initial_window_threads_overlay_opts() {
    let mut shell = MockShell::default();
    let anchor = crate::app::Anchor::BottomCenter { margin: 64.0 };
    let config = AppConfig::new("pill")
        .size(200, 48)
        .overlay(true)
        .anchor(anchor);
    let logical = LogicalSize::new(200.0, 48.0);

    create_initial_window(&mut shell, &config, logical).unwrap();

    assert_eq!(
        shell.calls,
        vec![CreateCall::With(
            "pill".to_owned(),
            logical,
            crate::app::WindowOpts {
                overlay: true,
                frameless: false,
                anchor: Some(anchor),
                user_zoom: None,
                ..crate::app::WindowOpts::default()
            }
        )]
    );
}

#[test]
fn initial_window_threads_user_zoom() {
    let mut shell = MockShell::default();
    let config = AppConfig::new("zoom").user_zoom(1.1);
    let logical = LogicalSize::new(720.0, 480.0);

    create_initial_window(&mut shell, &config, logical).unwrap();

    assert_eq!(
        shell.calls,
        vec![CreateCall::With(
            "zoom".to_owned(),
            logical,
            crate::app::WindowOpts {
                user_zoom: Some(1.1),
                ..crate::app::WindowOpts::default()
            }
        )]
    );
}

#[test]
fn initial_window_threads_frameless_opts() {
    let mut shell = MockShell::default();
    let config = AppConfig::new("frameless").size(1280, 720).frameless(true);
    let logical = LogicalSize::new(1280.0, 720.0);

    create_initial_window(&mut shell, &config, logical).unwrap();

    assert_eq!(
        shell.calls,
        vec![CreateCall::With(
            "frameless".to_owned(),
            logical,
            crate::app::WindowOpts {
                overlay: false,
                frameless: true,
                anchor: None,
                user_zoom: None,
                ..crate::app::WindowOpts::default()
            }
        )]
    );
}

#[test]
fn layout_for_scale_keeps_logical_rects_stable() {
    Runtime::scope("test", || {
        let mut cx = Cx::new_root();
        let root = cx.add(container().layout(LayoutStyle::block().fixed_size(720.0, 480.0)));
        let logical = LogicalSize::new(720.0, 480.0);
        let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));

        layout_for_scale(&mut cx, root, &mut layout, Scale::ONE, logical).unwrap();
        let one_x = layout.result(root).unwrap().rect();

        layout_for_scale(&mut cx, root, &mut layout, Scale::new(2.0), logical).unwrap();
        let two_x = layout.result(root).unwrap().rect();

        assert_eq!(one_x, two_x);
        assert_eq!(two_x.width.to_bits(), 720.0_f32.to_bits());
        assert_eq!(two_x.height.to_bits(), 480.0_f32.to_bits());
    });
}

#[test]
fn host_layout_matches_layout_for_scale() {
    Runtime::scope("host-layout", || {
        let mut host = crate::Host::new();
        host.mount(move || container().layout(LayoutStyle::block().fixed_size(720.0, 480.0)));
        let root = host.layout((720.0, 480.0)).unwrap().unwrap();
        let (_, engine, _) = host.tree().unwrap();
        let rect = engine.result(root).unwrap().rect();
        assert_eq!(rect.width.to_bits(), 720.0_f32.to_bits());
        assert_eq!(rect.height.to_bits(), 480.0_f32.to_bits());
    });
}

#[test]
fn layout_gate_skips_clean_input_and_runs_only_for_dirty_frames() {
    Runtime::scope("layout_gate", || {
        let mut passes = 0;

        Runtime::begin_effect();
        Runtime::begin_layout();
        assert!(!run_layout_if_dirty(false, || passes += 1));
        Runtime::begin_paint();
        Runtime::end_frame();
        assert_eq!(passes, 0, "clean input must not enter layout");

        Runtime::begin_effect();
        Runtime::mark_frame_dirty();
        Runtime::begin_layout();
        assert!(run_layout_if_dirty(Runtime::frame_dirty(), || passes += 1));
        Runtime::begin_paint();
        Runtime::end_frame();

        Runtime::begin_effect();
        Runtime::begin_layout();
        assert!(run_layout_if_dirty(true, || passes += 1));
        Runtime::begin_paint();
        Runtime::end_frame();
        assert_eq!(passes, 2);
    });
}

#[test]
fn animation_clock_advances_until_idle_and_rearms() {
    Runtime::scope("animation_clock_advances_until_idle_and_rearms", || {
        MotionPrefs::set(false);
        let value = signal(0.0_f32);
        let config = config(presets::DEFAULT).expect("default animation preset");
        let mut animator = Animator::new(value, config);
        let mut clock = AnimationClock::default();
        let mut now = Instant::now();

        animator.target(1.0);
        clock.tick_active(now);
        assert!(value.peek() > 0.0, "first live frame advances motion");

        for _ in 0..30 {
            now += Duration::from_millis(16);
            clock.tick_active(now);
            if active_count() == 0 {
                break;
            }
        }
        assert_eq!(active_count(), 0, "settled motion stops frame demand");
        assert!(clock.previous.is_none(), "idle clock drops its timestamp");

        animator.target(0.0);
        assert_eq!(active_count(), 1, "settled handle re-arms on a new target");
        clock.tick_active(now + Duration::from_millis(16));
        assert!(
            value.peek() < 1.0,
            "re-armed motion advances on the next frame"
        );
    });
}
