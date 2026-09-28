use super::*;

#[test]
fn spawned_task_completes_across_frames() {
    Runtime::scope("tui-spawn", || {
        let mut async_rt = crate::tasks::AsyncRuntime::new().expect("async runtime");
        async_rt.install_current();
        let result = crate::tasks::spawn::<u32, core::convert::Infallible, _>(async { Ok(7) });
        let mut host = Host::cells();
        host.mount(crate::kernel::text("hello"));
        let mut router = EventRouter::new();
        for _ in 0..100 {
            let _ = async_rt.pump();
            let _ = step(&mut host, &mut router, [], 40, 8);
            if result.peek().is_ok() {
                break;
            }
        }
        assert!(matches!(result.peek(), crate::tasks::Status::Ok(7)));
    });
}

#[test]
fn clean_frame_writes_no_terminal_bytes() {
    let mut screen = Screen::new(1, 1);
    screen.flush(&mut Vec::new()).expect("prime screen");
    let mut bytes = Vec::new();
    paint_frame(&mut bytes, &mut screen, false).expect("paint");
    assert!(bytes.is_empty(), "idle frame wrote {bytes:?}");
}

#[test]
fn step_paints_the_hud_like_the_live_host() {
    let _alone = crate::hud::TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Runtime::scope("tui-step-hud", || {
        crate::hud::set(true);
        let mut host = Host::cells();
        host.mount(crate::kernel::text("hello"));
        let mut router = EventRouter::new();
        crate::profile::reset();
        crate::profile::begin_frame(crate::profile::BackendStatus::TERMINAL);
        crate::profile::tick_frame();
        let text = step(&mut host, &mut router, [], 80, 8)
            .map(|grid| grid.text())
            .unwrap_or_default();
        crate::hud::set(false);
        assert!(text.contains('▁') || text.contains("idle"), "{text}");
    });
}

fn tab() -> ShellEvent {
    ShellEvent::Key(crate::hal::KeyEvent::new(crate::hal::KeyEventInit {
        key: crate::hal::Key::Tab.into(),
        physical: crate::hal::PhysicalKey::Tab,
        modifiers: crate::hal::ModifiersState::empty(),
        repeat: false,
        phase: crate::hal::KeyPhase::Down,
        text: None,
    }))
}

#[test]
fn the_cursor_sits_on_the_focused_control() {
    Runtime::scope("tui-focus-cursor", || {
        let mut host = Host::cells();
        host.mount(
            crate::kernel::container()
                .child(crate::kernel::text("title"))
                .child(crate::widgets::button("Save")),
        );
        let mut router = EventRouter::new();
        let _ = step(&mut host, &mut router, [], 40, 6);
        let grid = step(&mut host, &mut router, [tab()], 40, 6).expect("frame");
        let (col, row) = grid.cursor().expect("focus places the cursor");
        let line = grid.text().lines().nth(row).unwrap_or_default().to_owned();
        let from_cursor: String = line.chars().skip(col).collect();
        assert!(from_cursor.starts_with("Save"), "{line:?} at {col}");
    });
}

#[test]
fn an_announcement_reaches_the_status_row() {
    Runtime::scope("tui-announce", || {
        let visible = crate::reactive::signal(false);
        let mut host = Host::cells();
        host.mount(
            crate::kernel::container().child(crate::widgets::modal(visible).title("File saved")),
        );
        let mut router = EventRouter::new();
        let _ = step(&mut host, &mut router, [], 40, 8);
        visible.set(true);
        let grid = step(&mut host, &mut router, [], 40, 8).expect("frame");
        let text = grid.text();
        let last = text.lines().last().unwrap_or_default();
        assert!(last.starts_with("File saved"), "{text}");
    });
}
