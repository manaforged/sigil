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

#[test]
fn a_shell_started_in_a_folder_runs_commands_there() {
    Owner::scope("terminal_session", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        runtime.install_current();
        let dir = std::env::current_dir().expect("cwd").join("src");
        let session = Session::spawn_in("/bin/sh", &dir).expect("spawn");
        session.write(b"pwd\r").expect("write");
        let deadline = Instant::now() + Duration::from_secs(10);
        let wanted = dir.display().to_string();
        while Instant::now() < deadline && !session.lines.get().iter().any(|line| line.trim() == wanted) {
            runtime.pump();
            std::thread::sleep(Duration::from_millis(20));
        }
        let screen = session.lines.get();
        assert!(screen.iter().any(|line| line.trim() == wanted), "{wanted} not in {screen:?}");
    });
}
