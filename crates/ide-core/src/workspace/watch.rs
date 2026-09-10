use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use notify::event::{EventKind, ModifyKind, RenameMode};
use notify::{Event as NotifyEvent, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};

const DEBOUNCE_WINDOW: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
    Renamed(PathBuf, PathBuf),
}

impl FsEvent {
    const fn kind_tag(&self) -> u8 {
        match self {
            Self::Created(_) => 0,
            Self::Modified(_) => 1,
            Self::Removed(_) => 2,
            Self::Renamed(_, _) => 3,
        }
    }

    fn primary_path(&self) -> &Path {
        match self {
            Self::Created(p) | Self::Modified(p) | Self::Removed(p) => p,
            Self::Renamed(_, to) => to,
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct Watcher {
    rx: Receiver<FsEvent>,
    notify: Option<RecommendedWatcher>,
    debouncer: Option<thread::JoinHandle<()>>,
}

impl Watcher {
    pub fn start(root: &Path) -> std::io::Result<Self> {
        Self::start_with_window(root, DEBOUNCE_WINDOW)
    }

    pub(crate) fn start_with_window(root: &Path, window: Duration) -> std::io::Result<Self> {
        let (raw_tx, raw_rx) = mpsc::channel::<NotifyEvent>();
        let (out_tx, out_rx) = unbounded::<FsEvent>();
        let mut notify_watcher =
            notify::recommended_watcher(move |res: Result<NotifyEvent, notify::Error>| {
                if let Ok(ev) = res {
                    let _ = raw_tx.send(ev);
                }
            })
            .map_err(notify_to_io)?;
        notify_watcher
            .watch(root, RecursiveMode::Recursive)
            .map_err(notify_to_io)?;
        let debouncer = thread::Builder::new()
            .name("sigil-fs-debounce".into())
            .spawn(move || debounce_loop(raw_rx, out_tx, window))?;
        Ok(Self {
            rx: out_rx,
            notify: Some(notify_watcher),
            debouncer: Some(debouncer),
        })
    }

    #[must_use]
    pub fn next(&self) -> Option<FsEvent> {
        let _ = TryRecvError::Empty;
        self.rx.try_recv().ok()
    }

    #[must_use]
    pub fn wait(&self) -> Option<FsEvent> {
        self.rx.recv().ok()
    }

    #[must_use]
    pub fn wait_timeout(&self, timeout: Duration) -> Option<FsEvent> {
        self.rx.recv_timeout(timeout).ok()
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.notify.take();
        if let Some(handle) = self.debouncer.take() {
            let _ = handle.join();
        }
    }
}

fn notify_to_io(err: notify::Error) -> std::io::Error {
    std::io::Error::other(err)
}

#[allow(clippy::needless_pass_by_value)]
fn debounce_loop(raw_rx: mpsc::Receiver<NotifyEvent>, out_tx: Sender<FsEvent>, window: Duration) {
    let mut pending: HashMap<(u8, PathBuf), (FsEvent, Instant)> = HashMap::new();
    loop {
        let recv = if pending.is_empty() {
            raw_rx.recv().ok()
        } else {
            raw_rx.recv_timeout(window).ok()
        };
        if let Some(raw) = recv {
            for ev in translate(raw) {
                let key = (ev.kind_tag(), ev.primary_path().to_path_buf());
                pending.insert(key, (ev, Instant::now()));
            }
            continue;
        }
        if raw_rx.try_recv().is_err() && pending.is_empty() && drained(&raw_rx) {
            break;
        }
        if !flush_ready(&mut pending, &out_tx, window) {
            break;
        }
    }
    for (_, (ev, _)) in pending.drain() {
        let _ = out_tx.send(ev);
    }
}

fn drained(rx: &mpsc::Receiver<NotifyEvent>) -> bool {
    matches!(rx.try_recv(), Err(mpsc::TryRecvError::Disconnected))
}

fn flush_ready(
    pending: &mut HashMap<(u8, PathBuf), (FsEvent, Instant)>,
    out_tx: &Sender<FsEvent>,
    window: Duration,
) -> bool {
    let now = Instant::now();
    let ready: Vec<(u8, PathBuf)> = pending
        .iter()
        .filter(|(_, (_, t))| now.duration_since(*t) >= window)
        .map(|(k, _)| k.clone())
        .collect();
    for key in ready {
        if let Some((ev, _)) = pending.remove(&key)
            && out_tx.send(ev).is_err()
        {
            return false;
        }
    }
    true
}

fn translate(raw: NotifyEvent) -> Vec<FsEvent> {
    let paths = raw.paths;
    match raw.kind {
        EventKind::Create(_) => paths.into_iter().map(FsEvent::Created).collect(),
        EventKind::Remove(_) => paths.into_iter().map(FsEvent::Removed).collect(),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => translate_rename_pair(&paths),
        EventKind::Modify(_) | EventKind::Access(_) | EventKind::Other => {
            paths.into_iter().map(FsEvent::Modified).collect()
        }
        EventKind::Any => paths.into_iter().map(FsEvent::Modified).collect(),
    }
}

fn translate_rename_pair(paths: &[PathBuf]) -> Vec<FsEvent> {
    if let (Some(from), Some(to)) = (paths.first(), paths.get(1)) {
        return vec![FsEvent::Renamed(from.clone(), to.clone())];
    }
    paths.iter().cloned().map(FsEvent::Modified).collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#[path = "watch_tests.rs"]
mod tests;
