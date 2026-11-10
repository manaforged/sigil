#![cfg(unix)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::{Duration, Instant};

use sigil::app::Session;
use sigil::reactive::Owner;
use sigil::tasks::AsyncRuntime;

fn wait_for(
    runtime: &mut AsyncRuntime,
    session: &Session,
    done: impl Fn(&[String]) -> bool,
) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !done(&session.lines.get()) {
        runtime.pump();
        std::thread::sleep(Duration::from_millis(20));
    }
    session.lines.get()
}

fn shell(runtime: &mut AsyncRuntime) -> Session {
    shell_in(runtime, &std::env::current_dir().expect("cwd"))
}

fn shell_in(runtime: &mut AsyncRuntime, dir: &std::path::Path) -> Session {
    runtime.install_current();
    let session = Session::spawn_in("/bin/sh", dir).expect("spawn");
    let prompt = |line: &String| line.trim_end().ends_with(['$', '#']);
    let screen = wait_for(runtime, &session, |lines| lines.iter().any(prompt));
    assert!(screen.iter().any(prompt), "no prompt in {screen:?}");
    session
}

#[test]
fn a_shell_started_in_a_folder_runs_commands_there() {
    Owner::scope("terminal_session", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let dir = std::env::current_dir().expect("cwd").join("src");
        let session = shell_in(&mut runtime, &dir);
        session.write(b"pwd\r").expect("write");
        let wanted = dir.display().to_string();
        let screen = wait_for(&mut runtime, &session, |lines| {
            lines.iter().any(|line| line.trim() == wanted)
        });
        assert!(
            screen.iter().any(|line| line.trim() == wanted),
            "{wanted} not in {screen:?}"
        );
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
        let screen = wait_for(&mut runtime, &session, |lines| {
            lines.iter().any(|line| line.trim() == "30 100")
        });
        assert!(
            screen.iter().any(|line| line.trim() == "30 100"),
            "no 30 100 in {screen:?}"
        );
    });
}

#[test]
fn paste_reaches_the_program_as_typed_text() {
    Owner::scope("terminal_paste", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        session.paste("echo pasted-$((40+2))\r").expect("paste");
        let screen = wait_for(&mut runtime, &session, |lines| {
            lines.iter().any(|line| line.trim() == "pasted-42")
        });
        assert!(
            screen.iter().any(|line| line.trim() == "pasted-42"),
            "{screen:?}"
        );
    });
}

#[test]
fn scrolling_up_shows_lines_that_left_the_screen() {
    Owner::scope("terminal_scroll", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        session
            .write(b"i=1; while [ $i -le 60 ]; do echo line-$i; i=$((i+1)); done\r")
            .expect("write");
        let live = wait_for(&mut runtime, &session, |lines| {
            lines.iter().any(|line| line.trim() == "line-60")
        });
        assert!(
            !live.iter().any(|line| line.trim() == "line-1"),
            "line-1 is still on screen: {live:?}"
        );
        session.scroll(100).expect("scroll");
        let back = wait_for(&mut runtime, &session, |lines| {
            lines.iter().any(|line| line.trim() == "line-1")
        });
        assert!(
            back.iter().any(|line| line.trim() == "line-1"),
            "scrollback lost line-1: {back:?}"
        );
        session.scroll(-100).expect("scroll");
        let again = wait_for(&mut runtime, &session, |lines| {
            lines.iter().any(|line| line.trim() == "line-60")
        });
        assert!(
            again.iter().any(|line| line.trim() == "line-60"),
            "{again:?}"
        );
    });
}

fn screen_has(lines: &[String], wanted: &str) -> bool {
    lines.iter().any(|line| line.contains(wanted))
}

fn run(runtime: &mut AsyncRuntime, session: &Session, command: &str, wanted: &str) -> Vec<String> {
    session.write(command.as_bytes()).expect("write");
    let screen = wait_for(runtime, session, |lines| screen_has(lines, wanted));
    assert!(screen_has(&screen, wanted), "no {wanted} in {screen:?}");
    screen
}

const ASK: &str = "stty raw -echo; printf '%b' \"$QUERY\"; r=$(dd bs=1 count=$COUNT 2>/dev/null | od -An -c | tr -d ' \\n'); stty sane; echo got-$r\r";

fn ask(runtime: &mut AsyncRuntime, session: &Session, query: &str, count: usize, wanted: &str) {
    session
        .write(format!("QUERY='{query}' COUNT={count}\r").as_bytes())
        .expect("write");
    run(runtime, session, ASK, &format!("got-{wanted}"));
}

#[test]
fn a_program_that_asks_where_the_cursor_is_gets_an_answer() {
    Owner::scope("terminal_cursor_query", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        ask(&mut runtime, &session, "\\033[1;1H\\033[6n", 6, "033[1;1R");
    });
}

#[test]
fn a_program_that_asks_what_terminal_this_is_gets_an_answer() {
    Owner::scope("terminal_identity", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        ask(&mut runtime, &session, "\\033[c", 9, "033[?62;22c");
    });
}

#[test]
fn a_program_that_asks_for_the_background_color_gets_one() {
    Owner::scope("terminal_background", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        ask(&mut runtime, &session, "\\033]11;?\\033\\\\", 5, "033]11");
    });
}

#[test]
fn the_title_a_program_sets_is_published() {
    Owner::scope("terminal_title", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        run(
            &mut runtime,
            &session,
            "printf '\\033]2;my-title\\007'; echo set\r",
            "set",
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && session.title.get() != "my-title" {
            runtime.pump();
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(session.title.get(), "my-title");
    });
}

#[test]
fn a_bell_is_counted() {
    Owner::scope("terminal_bell", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        run(
            &mut runtime,
            &session,
            "printf '\\007'; echo rang\r",
            "rang",
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && session.bell.get() == 0 {
            runtime.pump();
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(session.bell.get(), 1);
    });
}

#[test]
fn a_program_that_exits_reports_its_code() {
    Owner::scope("terminal_exit", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        session.write(b"exit 7\r").expect("write");
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && session.exited.get().is_none() {
            runtime.pump();
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(session.exited.get(), Some(7));
    });
}

#[test]
fn a_program_is_told_this_terminal_has_full_color() {
    Owner::scope("terminal_color", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        run(
            &mut runtime,
            &session,
            "echo color-$COLORTERM\r",
            "color-truecolor",
        );
    });
}

#[test]
fn the_default_shell_is_a_login_shell_in_the_folder() {
    Owner::scope("terminal_login", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        runtime.install_current();
        let dir = std::env::current_dir().expect("cwd");
        let session = Session::shell_in(&dir).expect("spawn");
        run(
            &mut runtime,
            &session,
            "printf 'zero-%s\\n' \"$0\"\r",
            "zero--",
        );
    });
}

#[test]
fn scrolled_back_output_stays_put_until_a_key() {
    Owner::scope("terminal_scroll_hold", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        let session = shell(&mut runtime);
        session
            .write(b"i=1; while [ $i -le 60 ]; do echo line-$i; i=$((i+1)); done\r")
            .expect("write");
        wait_for(&mut runtime, &session, |lines| screen_has(lines, "line-60"));
        session.scroll(100).expect("scroll");
        wait_for(&mut runtime, &session, |lines| {
            lines.iter().any(|line| line.trim() == "line-1")
        });
        session.write(b"sleep 0; echo more\r").expect("write");
        std::thread::sleep(Duration::from_millis(300));
        runtime.pump();
        assert!(
            !screen_has(&session.lines.get(), "more"),
            "output pulled the view back: {:?}",
            session.lines.get()
        );
    });
}
