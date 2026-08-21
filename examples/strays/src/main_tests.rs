use std::time::Duration;

use super::*;
use crate::scan::{Stray, Verdict};

fn row(age: u64, tty: bool, cwd: &str) -> Stray {
    Stray {
        pid: 42,
        pgid: 42,
        ports: vec![5174],
        age: Duration::from_secs(age),
        rss: 0,
        cwd: Some(PathBuf::from(cwd)),
        cmd: String::from("node vite"),
        tty,
    }
}

const LIMIT: Duration = Duration::from_secs(3600);

#[test]
fn a_watched_server_is_never_a_stray() {
    let root = PathBuf::from("/dev");
    assert_eq!(
        row(99_999, true, "/dev/app").verdict(&root, LIMIT),
        Verdict::Held
    );
}

#[test]
fn a_system_daemon_outlives_any_limit_without_becoming_a_stray() {
    let root = PathBuf::from("/dev");
    assert_eq!(
        row(999_999, false, "/usr/libexec").verdict(&root, LIMIT),
        Verdict::System
    );
}

#[test]
fn an_unattended_dev_server_past_the_limit_is_a_stray() {
    let root = PathBuf::from("/dev");
    assert_eq!(
        row(7200, false, "/dev/app").verdict(&root, LIMIT),
        Verdict::Stray
    );
}

#[test]
fn a_young_unattended_dev_server_is_still_fresh() {
    let root = PathBuf::from("/dev");
    assert_eq!(
        row(60, false, "/dev/app").verdict(&root, LIMIT),
        Verdict::Fresh
    );
}
