use super::*;
use crate::kernel::HostMode;

fn mounted<N: IntoNode>(tree: impl Fn() -> N + 'static) -> Host {
    let mut host = Host::new(HostMode::Cell);
    host.mount(tree);
    host
}

#[test]
fn spawned_task_completes_across_frames() {
    Owner::scope("tui-spawn", || {
        let mut async_rt = crate::tasks::AsyncRuntime::new().expect("async runtime");
        let (wake, ready) = std::sync::mpsc::channel();
        async_rt.set_waker(move || wake.send(()).expect("notify the terminal test"));
        async_rt.install_current();
        let result = crate::tasks::spawn::<u32, core::convert::Infallible, _>(async { Ok(7) });
        let mut host = mounted(move || crate::kernel::text("hello"));
        step(&mut host, [], 40, 8).expect("frame before task delivery");
        ready
            .recv_timeout(Duration::from_secs(5))
            .expect("the completed task wakes the terminal");
        async_rt.pump();
        step(&mut host, [], 40, 8).expect("frame after task delivery");
        assert!(matches!(result.peek(), crate::tasks::Status::Ok(7)));
    });
}

#[test]
fn clean_frame_writes_no_terminal_bytes() {
    let mut screen = Screen::new(1, 1);
    screen.flush(&mut Vec::new()).expect("prime screen");
    let mut bytes = Vec::new();
    paint_frame(&mut bytes, &mut screen, false, false).expect("paint");
    assert!(bytes.is_empty(), "idle frame wrote {bytes:?}");
}

#[test]
fn step_paints_the_hud_like_the_live_host() {
    let _alone = crate::hud::TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Owner::scope("tui-step-hud", || {
        crate::hud::set(true);
        let mut host = mounted(move || crate::kernel::text("hello"));
        crate::profile::reset();
        crate::profile::begin_frame(crate::profile::BackendStatus::TERMINAL);
        crate::profile::tick_frame();
        let text = step(&mut host, [], 80, 8)
            .map(|grid| grid.text())
            .unwrap_or_default();
        crate::hud::set(false);
        assert!(text.contains('▁') || text.contains("idle"), "{text}");
    });
}

fn tab() -> Event {
    Event::key(crate::events::Key::Tab, crate::events::KeyPhase::Down)
        .physical(crate::events::PhysicalKey::Tab)
}

#[test]
fn the_cursor_sits_on_the_focused_control() {
    Owner::scope("tui-focus-cursor", || {
        let mut host = mounted(move || {
            crate::kernel::container()
                .child(crate::kernel::text("title"))
                .child(crate::widgets::button("Save"))
        });
        let _ = step(&mut host, [], 40, 6);
        let grid = step(&mut host, [tab()], 40, 6).expect("frame");
        let (col, row) = grid.cursor().expect("focus places the cursor");
        let line = grid.text().lines().nth(row).unwrap_or_default().to_owned();
        let from_cursor: String = line.chars().skip(col).collect();
        assert!(from_cursor.starts_with("Save"), "{line:?} at {col}");
    });
}

#[test]
fn an_announcement_reaches_the_status_row() {
    Owner::scope("tui-announce", || {
        let visible = crate::reactive::signal(false);
        let mut host = mounted(move || {
            crate::kernel::container().child(crate::widgets::modal(visible).title("File saved"))
        });
        let mut screen = Screen::new(40, 8);
        let _ = screen.frame(&mut host, [], (40, 8));
        visible.set(true);
        let text = screen.frame(&mut host, [], (40, 8)).expect("frame").text();
        let last = text.lines().last().unwrap_or_default();
        assert!(last.starts_with("File saved"), "{text}");
    });
}

#[test]
fn a_status_region_on_screen_at_start_is_not_announced() {
    Owner::scope("tui-quiet-start", || {
        let mut host = mounted(move || {
            crate::kernel::container()
                .child(crate::kernel::text("body"))
                .child(crate::widgets::status_bar([crate::kernel::text("Ready")]))
        });
        let mut screen = Screen::new(40, 8);
        let first = screen
            .frame(&mut host, [], (40, 8))
            .expect("first frame")
            .text();
        assert!(first.contains("Ready"), "{first}");
        let next = screen
            .frame(&mut host, [], (40, 8))
            .expect("next frame")
            .text();
        assert!(
            !first.contains("Status bar") && !next.contains("Status bar"),
            "{first}\n{next}"
        );
    });
}
