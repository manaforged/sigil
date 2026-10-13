#![allow(clippy::unwrap_used, clippy::shadow_unrelated)]

use std::fmt;
use std::time::{Duration, Instant};

use crate::animation_runtime::{Animator, MotionPrefs, active_count, config, presets};
use crate::events::ShellEvent;
use crate::geometry::{LogicalSize, PhysicalSize};
use crate::hal::Shell;
use crate::kernel::container;
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Runtime, signal};
use crate::tasks::{AsyncRuntime, signal_writer};

use crate::Host;
use crate::app::config::AppConfig;
use crate::app::launch::{AnimationClock, create_initial_window};

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
        let shows = |host: &Host, text: &str| {
            host.accessibility_tree(&router)
                .nodes
                .iter()
                .any(|(_, node)| node.value() == Some(text))
        };
        assert!(shows(&host, "Profile 0"));
        let mut async_rt = AsyncRuntime::new().unwrap();
        async_rt.install_current();
        signal_writer(revision).set(1).unwrap();

        Runtime::begin_effect();
        let _ = async_rt.pump();
        Runtime::begin_layout();
        if Runtime::frame_dirty() {
            host.layout((200.0, 100.0)).unwrap();
        }
        Runtime::begin_paint();
        assert!(shows(&host, "Profile 1"));
        assert!(!shows(&host, "Profile 0"));
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
fn initial_window_threads_config_into_logical_size_and_opts() {
    let anchor = crate::app::Anchor::BottomCenter { margin: 64.0 };
    let opts = crate::app::WindowOpts::default;
    let cases = [
        (AppConfig::new("test").size(720, 480), opts()),
        (
            AppConfig::new("pill")
                .size(200, 48)
                .overlay(true)
                .anchor(anchor),
            crate::app::WindowOpts {
                overlay: true,
                anchor: Some(anchor),
                ..opts()
            },
        ),
        (
            AppConfig::new("zoom").user_zoom(1.1),
            crate::app::WindowOpts {
                user_zoom: Some(1.1),
                ..opts()
            },
        ),
        (
            AppConfig::new("frameless").size(1280, 720).frameless(true),
            crate::app::WindowOpts {
                frameless: true,
                ..opts()
            },
        ),
    ];
    for (config, expected) in cases {
        let mut shell = MockShell::default();
        let logical = super::logical_size_from_config(config.window_size());
        let handle = create_initial_window(&mut shell, &config, logical).unwrap();
        assert_eq!(handle, 0);
        assert_eq!(
            shell.calls,
            vec![CreateCall::With(
                config.title().to_owned(),
                logical,
                expected
            )]
        );
    }
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
fn host_layout_keeps_the_logical_viewport() {
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

#[test]
fn the_terminal_host_paints_at_sixty_hertz_by_default() {
    let config = crate::AppConfig::new("hz");
    assert_eq!(config.frame_interval, Duration::from_micros(16_667));
}
