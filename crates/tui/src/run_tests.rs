use super::*;

#[test]
fn spawned_task_completes_across_frames() {
    Runtime::scope("tui-spawn", || {
        let mut async_rt = sigil::tasks::AsyncRuntime::new().expect("async runtime");
        async_rt.install_current();
        let result = sigil::tasks::spawn::<u32, core::convert::Infallible, _>(async { Ok(7) });
        let mut host = Host::cells();
        host.mount(sigil::kernel::text("hello"));
        let mut router = EventRouter::new();
        for _ in 0..100 {
            let _ = async_rt.pump();
            let _ = step(&mut host, &mut router, [], 40, 8);
            if result.peek().is_ok() {
                break;
            }
        }
        assert!(matches!(result.peek(), sigil::tasks::Status::Ok(7)));
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
