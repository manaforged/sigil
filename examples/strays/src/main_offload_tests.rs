use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::scan::Scanner;
use crate::state::AppState;

/// A refresh runs off the frame thread and its rows reach the table.
///
/// The scan forks `ps` and walks the process table. Doing that between two
/// frames is what made this app freeze on its own first frame.
#[test]
fn a_refresh_lands_without_blocking_the_frame() {
    sigil::prelude::Runtime::scope("offload", || {
        let reader = Arc::new(Mutex::new(Scanner::new()));
        let state = AppState::adopt(
            Arc::clone(&reader),
            Vec::new(),
            PathBuf::from(scan::DEV),
            scan::LIMIT,
        );
        assert!(state.table.rows().peek().is_empty(), "starts with nothing");

        state.refresh();
        // The frame thread is free while the worker reads the machine.
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
        assert!(landed, "the scan's rows must reach the table");
    });
}
