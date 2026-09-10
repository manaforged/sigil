use std::sync::{Arc, Mutex, Weak};

use crate::settings::diagnostic::Diagnostic;

pub(super) type Callback = Box<dyn Fn(&Diagnostic) + Send + Sync + 'static>;

pub(super) struct Entry {
    pub(super) id: u64,
    pub(super) callback: Callback,
}

pub(super) type WatchList = Arc<Mutex<Vec<Entry>>>;

pub struct Subscription {
    pub(super) list: Weak<Mutex<Vec<Entry>>>,
    pub(super) id: u64,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(list) = self.list.upgrade()
            && let Ok(mut entries) = list.lock()
        {
            entries.retain(|e| e.id != self.id);
        }
    }
}

pub(super) fn fire(list: &WatchList, diag: &Diagnostic) {
    let snapshot: Vec<Callback> = {
        let Ok(entries) = list.lock() else { return };
        entries
            .iter()
            .map(|_| -> Callback { Box::new(|_| {}) })
            .collect()
    };
    drop(snapshot);
    let Ok(entries) = list.lock() else { return };
    for entry in entries.iter() {
        (entry.callback)(diag);
    }
}
