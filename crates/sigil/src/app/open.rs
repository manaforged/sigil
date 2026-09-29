use std::ffi::OsStr;
use std::process::Command;

use crate::{AppResult, Error};

pub fn open(target: impl AsRef<OsStr>) -> AppResult<()> {
    if !allowed(target.as_ref()) {
        return Err(Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "open accepts only http, https, and mailto URLs",
        )));
    }
    let mut child = command(target.as_ref()).spawn().map_err(Error::io)?;
    std::thread::spawn(move || drop(child.wait()));
    Ok(())
}

fn allowed(target: &OsStr) -> bool {
    let Some(text) = target.to_str() else {
        return false;
    };
    let lower = text.to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
}

pub(crate) fn command(target: &OsStr) -> Command {
    #[cfg(target_os = "macos")]
    let mut cmd = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut cmd = Command::new("rundll32");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = Command::new("xdg-open");

    #[cfg(target_os = "windows")]
    cmd.arg("url.dll,FileProtocolHandler");
    cmd.arg(target);
    cmd
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod tests;
