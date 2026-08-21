use std::time::Duration;

use super::*;
use crate::scan::Stray;
use crate::state::AppState;

const LIMIT: Duration = Duration::from_secs(3600);

fn row(pid: u32, age: u64, tty: bool, cwd: Option<&str>) -> Stray {
    Stray {
        pid,
        pgid: pid,
        ports: vec![1000],
        age: Duration::from_secs(age),
        rss: 0,
        cwd: cwd.map(PathBuf::from),
        cmd: format!("proc{pid}"),
        tty,
    }
}

/// Every row's badge must sit on that row's line.
///
/// This is the fixture that first showed the chip column painting a row
/// low, and it still fails if the header is allowed to shrink.
#[test]
fn each_row_paints_its_own_state() {
    let rows = vec![
        row(1111, 99_999, false, None),           // system
        row(2222, 99_999, false, Some("/dev/a")), // STRAY
        row(3333, 99_999, true, Some("/dev/b")),  // held
        row(4444, 10, false, Some("/dev/c")),     // fresh
    ];
    let grid = sigil_tui::present(
        move |cx: &mut Cx| {
            let state = AppState::from_rows(rows.clone(), PathBuf::from("/dev"), LIMIT);
            cx.add(view::app_tree(&state))
        },
        120,
        32,
        &[],
    )
    .expect("layout");
    println!("{grid}");
    for (pid, want) in [
        ("1111", "system"),
        ("2222", "STRAY"),
        ("3333", "held"),
        ("4444", "fresh"),
    ] {
        let line = grid
            .lines()
            .find(|line| line.contains(pid))
            .unwrap_or_else(|| panic!("no line for {pid} in:\n{grid}"));
        assert!(
            line.contains(want),
            "row {pid} must paint {want}, got:\n{line}\nfull:\n{grid}"
        );
    }
}
