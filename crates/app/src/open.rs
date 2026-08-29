use std::ffi::OsStr;
use std::process::Command;

use crate::{AppResult, Error};

pub fn open(target: impl AsRef<OsStr>) -> AppResult<()> {
    let mut child = command(target.as_ref()).spawn().map_err(Error::new)?;
    std::thread::spawn(move || drop(child.wait()));
    Ok(())
}

pub(crate) fn command(target: &OsStr) -> Command {
    #[cfg(target_os = "macos")]
    let mut cmd = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut cmd = Command::new("cmd");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = Command::new("xdg-open");

    #[cfg(target_os = "windows")]
    cmd.args(["/C", "start", ""]);
    cmd.arg(target);
    cmd
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod tests;
