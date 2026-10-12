use std::ffi::OsStr;

use super::command;

#[test]
fn command_uses_platform_helper() {
    let cmd = command(OsStr::new("https://example.com"));
    let program = cmd.get_program();
    #[cfg(target_os = "macos")]
    assert_eq!(program, "open");
    #[cfg(target_os = "windows")]
    assert_eq!(program, "rundll32");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    assert_eq!(program, "xdg-open");
}

#[test]
fn metacharacters_stay_inside_one_argument() {
    let target = OsStr::new("https://example.com/?a=1&b=2");
    let cmd = command(target);
    assert!(
        cmd.get_args().any(|arg| arg == target),
        "the target must stay one argument"
    );
    #[cfg(target_os = "windows")]
    assert_ne!(cmd.get_program(), OsStr::new("cmd"));
}
