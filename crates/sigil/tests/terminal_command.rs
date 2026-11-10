#![cfg(unix)]

use std::process::Command;
use std::time::{Duration, Instant};

use sigil::app::Session;
use sigil::reactive::Owner;
use sigil::tasks::AsyncRuntime;

#[test]
fn a_command_runs_with_its_arguments_environment_and_folder() {
    Owner::scope("terminal_command", || {
        let mut runtime = AsyncRuntime::new().expect("runtime");
        runtime.install_current();
        let dir = std::env::current_dir().expect("cwd").join("src");
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "echo \"$GREETING in $(pwd)\""])
            .env("GREETING", "hello")
            .current_dir(&dir);
        let session = Session::command(&command).expect("spawn");
        let wanted = format!("hello in {}", dir.display());
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline
            && !session.lines.get().iter().any(|line| line.trim() == wanted)
        {
            runtime.pump();
            std::thread::sleep(Duration::from_millis(20));
        }
        let screen = session.lines.get();
        assert!(
            screen.iter().any(|line| line.trim() == wanted),
            "{wanted} not in {screen:?}"
        );
    });
}
