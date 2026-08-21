//! `Watcher` — debounced + coalesced fs-watch over `notify`.
//!
//! `notify` raises raw OS events: kqueue / inotify / `ReadDirectoryChangesW`
//! / `FSEvents` all emit multi-event bursts when an editor saves
//! one file (write, attribute change, close). The IDE wants one
//! event per logical change.
//!
//! Strategy:
//! 1. Spawn `notify::RecommendedWatcher` on a background thread.
//! 2. Each raw event is reduced to one `FsEvent` per `(kind, path)`
//!    pair within a 50ms sliding window via a debouncer thread.
//! 3. Coalesced events ship to the consumer via a
//!    `crossbeam-channel`. `next()` is non-blocking, `wait()` blocks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use notify::event::{EventKind, ModifyKind, RenameMode};
use notify::{Event as NotifyEvent, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};

/// Default coalescing window. 50ms is the sweet spot empirically:
/// long enough to collapse editor save-bursts, short enough that
/// the editor's "file changed externally" prompt feels live.
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(50);

/// One logical filesystem change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsEvent {
    /// A path appeared.
    Created(PathBuf),
    /// A path's contents or metadata changed.
    Modified(PathBuf),
    /// A path was removed.
    Removed(PathBuf),
    /// A path was renamed from `.0` to `.1`.
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

/// Owns a `notify` watcher thread + a debouncer thread. Dropping
/// the `Watcher` stops both.
///
/// Drop order is load-bearing: the `notify` watcher must be torn
/// down *before* the debouncer thread is joined. Dropping the
/// `notify` watcher cascades-drops the only `Sender<NotifyEvent>`
/// (held inside its callback), which causes the debouncer's
/// `raw_rx.recv()` to return `Err(Disconnected)` and exit cleanly.
/// If joined first, the debouncer blocks forever on the still-live
/// callback-sender clone. The `Option<>` wraps let `Drop` move-out.
#[allow(missing_debug_implementations)]
pub struct Watcher {
    rx: Receiver<FsEvent>,
    notify: Option<RecommendedWatcher>,
    debouncer: Option<thread::JoinHandle<()>>,
}

impl Watcher {
    /// Start watching `root` recursively. Forwards events through
    /// the debouncer; consumers receive coalesced events.
    pub fn start(root: &Path) -> std::io::Result<Self> {
        Self::start_with_window(root, DEBOUNCE_WINDOW)
    }

    /// Tests need a custom window so they can collapse bursts
    /// deterministically without waiting 50ms × N.
    pub(crate) fn start_with_window(root: &Path, window: Duration) -> std::io::Result<Self> {
        let (raw_tx, raw_rx) = mpsc::channel::<NotifyEvent>();
        let (out_tx, out_rx) = unbounded::<FsEvent>();
        // The closure is the *only* owner of `raw_tx` — when the
        // notify watcher drops, the closure drops, the sender drops,
        // and the debouncer's `raw_rx.recv()` unblocks with
        // `Disconnected`. No clone, no struct-field sender.
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

    /// Non-blocking poll. Returns `None` if no event is available.
    #[must_use]
    pub fn next(&self) -> Option<FsEvent> {
        // `try_recv()` returns Err on both Empty and Disconnected;
        // both collapse to `None` here.
        let _ = TryRecvError::Empty;
        self.rx.try_recv().ok()
    }

    /// Block until an event arrives. Returns `None` only if the
    /// watcher thread has exited (typically because the `Watcher`
    /// is being dropped).
    #[must_use]
    pub fn wait(&self) -> Option<FsEvent> {
        self.rx.recv().ok()
    }

    /// Block until an event arrives or `timeout` elapses.
    #[must_use]
    pub fn wait_timeout(&self, timeout: Duration) -> Option<FsEvent> {
        self.rx.recv_timeout(timeout).ok()
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        // Drop the notify watcher first — its callback owns the only
        // live `Sender<NotifyEvent>`. Dropping it disconnects the
        // debouncer's receiver so `recv()` returns immediately.
        self.notify.take();
        if let Some(handle) = self.debouncer.take() {
            let _ = handle.join();
        }
    }
}

fn notify_to_io(err: notify::Error) -> std::io::Error {
    std::io::Error::other(err)
}

/// Coalesce raw `notify` events into `FsEvent`s on a `window`-sized
/// sliding window. Each `(kind_tag, primary_path)` collapses to one
/// emit per window.
//
// Both channel endpoints are intentionally moved into this function:
// the receiver must live for the thread's lifetime (its drop closes
// the upstream channel) and the sender's drop signals consumer
// shutdown. A `&Receiver` would let the caller race the drop. Hence
// the targeted `needless_pass_by_value` allow.
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
            // Channel closed and nothing to flush — exit.
            break;
        }
        if !flush_ready(&mut pending, &out_tx, window) {
            break;
        }
    }
    // Final drain on shutdown.
    for (_, (ev, _)) in pending.drain() {
        let _ = out_tx.send(ev);
    }
}

fn drained(rx: &mpsc::Receiver<NotifyEvent>) -> bool {
    matches!(rx.try_recv(), Err(mpsc::TryRecvError::Disconnected))
}

/// Returns `false` if the output channel has closed (consumer
/// dropped the Watcher).
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
#[path = "watch_tests.rs"]
mod tests;
