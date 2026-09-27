use std::fmt;
use std::path::PathBuf;
use std::thread::JoinHandle;
use std::time::Duration;
use web_time::Instant;

use crate::app::{AppResult, Error};
use crate::reactive::Owner;
use crate::tasks::{Dispatcher, OwnerHandle, dispatch_to_owner};

const DEBOUNCE: Duration = Duration::from_millis(75);
const MAX_DEBOUNCE: Duration = Duration::from_secs(1);

pub fn watch_theme(path: impl Into<PathBuf>) -> AppResult<ThemeWatcher> {
    if !crate::tasks::installed() {
        return Err(Error::task(NoRuntime));
    }
    let dispatcher = crate::tasks::dispatcher();
    let owner = Owner::new("sigil::watch_theme", None);
    let target = OwnerHandle::from_id(owner.id());
    let path = path.into();
    let (watcher, rx) = crate::app::theme_file::watcher(&path).map_err(Error::io)?;
    let reloader = std::thread::spawn(move || reload(path, rx, dispatcher, target));
    Ok(ThemeWatcher {
        watcher: Some(watcher),
        reloader: Some(reloader),
        owner,
    })
}

#[must_use = "dropping a ThemeWatcher stops the theme reload watch"]
pub struct ThemeWatcher {
    watcher: Option<notify::RecommendedWatcher>,
    reloader: Option<JoinHandle<()>>,
    owner: Owner,
}

impl Drop for ThemeWatcher {
    fn drop(&mut self) {
        drop(self.watcher.take());
        if let Some(reloader) = self.reloader.take()
            && reloader.join().is_err()
        {
            tracing::warn!(target: "sigil::theme", "theme reloader thread panicked");
        }
        self.owner.dispose();
    }
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "the API takes ownership by contract at this boundary"
)]
fn reload(
    path: PathBuf,
    rx: crossbeam_channel::Receiver<()>,
    dispatcher: Dispatcher,
    owner: OwnerHandle,
) {
    while rx.recv().is_ok() {
        let window = Instant::now();
        while window.elapsed() < MAX_DEBOUNCE && rx.recv_timeout(DEBOUNCE).is_ok() {}
        let file = path.clone();
        dispatch_to_owner(&dispatcher, owner, move || {
            if let Some(theme) = crate::app::theme_file::load(&file) {
                crate::app::set_theme(theme);
            }
        });
    }
}

#[derive(Debug)]
struct NoRuntime;

impl fmt::Display for NoRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("watch_theme needs AsyncRuntime::install_current")
    }
}

impl std::error::Error for NoRuntime {}
