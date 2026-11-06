#![cfg(unix)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::{Duration, Instant};

use sigil::app::Session;
use sigil::reactive::Owner;
use sigil::tasks::AsyncRuntime;

fn wait_for(runtime: &mut AsyncRuntime, session: &Session, done: impl Fn(&[String]) -> bool) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !done(&session.lines.get()) {
        runtime.pump();
        std::thread::sleep(Duration::from_millis(20));
    }
    session.lines.get()
}

fn shell(runtime: &mut AsyncRuntime) -> Session {
    runtime.install_current();
    let dir = std::env::current_dir().expect("cwd");
    Session::spawn_in("/bin/sh", &dir).expect("spawn")
}

#[test]
fn a_shell_started_in_a_folder_runs_commands_there() {
    Owner::scope("terminal_session", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        runtime.install_current();
        let dir = std::env::current_dir().expect("cwd").join("src");
        let session = Session::spawn_in("/bin/sh", &dir).expect("spawn");
        session.write(b"pwd\r").expect("write");
        let wanted = dir.display().to_string();
        let screen = wait_for(&mut runtime, &session, |lines| lines.iter().any(|line| line.trim() == wanted));
        assert!(screen.iter().any(|line| line.trim() == wanted), "{wanted} not in {screen:?}");
    });
}

#[test]
fn a_resized_terminal_tells_the_program_its_new_size() {
    Owner::scope("terminal_resize", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        session.resize(100, 30).expect("resize");
        std::thread::sleep(Duration::from_millis(200));
        session.write(b"stty size\r").expect("write");
        let screen = wait_for(&mut runtime, &session, |lines| lines.iter().any(|line| line.trim() == "30 100"));
        assert!(screen.iter().any(|line| line.trim() == "30 100"), "no 30 100 in {screen:?}");
    });
}

#[test]
fn paste_reaches_the_program_as_typed_text() {
    Owner::scope("terminal_paste", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        session.paste("echo pasted-$((40+2))\r").expect("paste");
        let screen = wait_for(&mut runtime, &session, |lines| lines.iter().any(|line| line.trim() == "pasted-42"));
        assert!(screen.iter().any(|line| line.trim() == "pasted-42"), "{screen:?}");
    });
}

#[test]
fn scrolling_up_shows_lines_that_left_the_screen() {
    Owner::scope("terminal_scroll", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        session.write(b"i=1; while [ $i -le 60 ]; do echo line-$i; i=$((i+1)); done\r").expect("write");
        let live = wait_for(&mut runtime, &session, |lines| lines.iter().any(|line| line.trim() == "line-60"));
        assert!(!live.iter().any(|line| line.trim() == "line-1"), "line-1 is still on screen: {live:?}");
        session.scroll(100).expect("scroll");
        let back = wait_for(&mut runtime, &session, |lines| lines.iter().any(|line| line.trim() == "line-1"));
        assert!(back.iter().any(|line| line.trim() == "line-1"), "scrollback lost line-1: {back:?}");
        session.scroll(-100).expect("scroll");
        let again = wait_for(&mut runtime, &session, |lines| lines.iter().any(|line| line.trim() == "line-60"));
        assert!(again.iter().any(|line| line.trim() == "line-60"), "{again:?}");
    });
}
