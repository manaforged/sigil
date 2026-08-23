use std::ffi::OsStr;

use super::command;

#[test]
fn command_uses_platform_helper() {
    let cmd = command(OsStr::new("https://example.com"));
    let program = cmd.get_program();
    #[cfg(target_os = "macos")]
    assert_eq!(program, "open");
    #[cfg(target_os = "windows")]
    assert_eq!(program, "cmd");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    assert_eq!(program, "xdg-open");
}

#[test]
fn command_passes_the_target() {
    let cmd = command(OsStr::new("/tmp/file.txt"));
    let args: Vec<_> = cmd.get_args().collect();
    assert!(
        args.iter().any(|arg| *arg == OsStr::new("/tmp/file.txt")),
        "expected target in {args:?}"
    );
}
