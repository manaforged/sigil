use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::scan::Scanner;
use crate::state::AppState;

#[test]
fn a_refresh_lands_without_blocking_the_frame() {
    use std::io::BufRead as _;
    sigil::prelude::Runtime::scope("offload", || {
        let reader = Arc::new(Mutex::new(Scanner::new()));
        let state = AppState::adopt(
            Arc::clone(&reader),
            Vec::new(),
            scan::dev_root(),
            scan::LIMIT,
        );
        assert!(state.table.rows().peek().is_empty(), "starts with nothing");

        let mut listener = std::process::Command::new("python3")
            .arg("-c")
            .arg(
                "import socket,time; s=socket.socket(); s.bind(('127.0.0.1',0)); \
                  s.listen(1); print(s.getsockname()[1], flush=True); time.sleep(30)",
            )
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("spawn child listener");
        let port: u16 = {
            let mut line = String::new();
            std::io::BufReader::new(listener.stdout.take().expect("piped stdout"))
                .read_line(&mut line)
                .expect("read listener port");
            line.trim().parse().expect("listener port")
        };
        for _ in 0..100 {
            if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        state.refresh();
        let mut landed = false;
        for _ in 0..400 {
            std::thread::sleep(Duration::from_millis(5));
            sigil_tui::collect_work();
            state.land();
            if !state.table.rows().peek().is_empty() {
                landed = true;
                break;
            }
        }
        let _ = listener.kill();
        let _ = listener.wait();
        assert!(landed, "the scan's rows must reach the table");
    });
}
