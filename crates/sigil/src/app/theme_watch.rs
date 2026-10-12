use std::path::PathBuf;
use std::thread::JoinHandle;
use std::time::Duration;
use web_time::Instant;

use crate::reactive::Owner;
use crate::tasks::OwnerHandle;
use crate::tasks::dispatch::dispatch_to_owner;
use crate::tasks::runtime::Dispatcher;
use crate::{AppResult, Error};

const DEBOUNCE: Duration = Duration::from_millis(75);
const MAX_DEBOUNCE: Duration = Duration::from_secs(1);

pub fn watch_theme(path: impl Into<PathBuf>) -> AppResult<ThemeWatcher> {
    let Some(dispatcher) = crate::tasks::current::dispatcher() else {
        return Err(Error::task(std::io::Error::other(
            "watch_theme needs AsyncRuntime::install_current",
        )));
    };
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
