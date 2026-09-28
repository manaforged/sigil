use std::convert::Infallible;
use std::path::PathBuf;

use rfd::{AsyncFileDialog, AsyncMessageDialog, FileHandle, MessageButtons, MessageDialogResult};

use crate::reactive::Signal;
use crate::tasks::{Status, spawn};

use super::{AlertKind, FileDialog, PickKind, Picked};

fn builder(dialog: FileDialog) -> AsyncFileDialog {
    let mut inner = AsyncFileDialog::new();
    for filter in &dialog.filters {
        inner = inner.add_filter(filter.name.clone(), &filter.extensions);
    }
    if let Some(dir) = dialog.directory {
        inner = inner.set_directory(dir);
    }
    if let Some(name) = dialog.file_name {
        inner = inner.set_file_name(name);
    }
    inner
}

fn path(handle: &FileHandle) -> PathBuf {
    handle.path().to_path_buf()
}

pub(super) fn pick_one(dialog: FileDialog, kind: PickKind) -> Picked<PathBuf> {
    let inner = builder(dialog);
    match kind {
        PickKind::Folder => {
            let task = inner.pick_folder();
            spawn(async move { Ok(task.await.map(|handle| path(&handle))) })
        }
        PickKind::Save => {
            let task = inner.save_file();
            spawn(async move { Ok(task.await.map(|handle| path(&handle))) })
        }
        PickKind::File | PickKind::Files => {
            let task = inner.pick_file();
            spawn(async move { Ok(task.await.map(|handle| path(&handle))) })
        }
    }
}

pub(super) fn open_files(dialog: FileDialog) -> Picked<Vec<PathBuf>> {
    let task = builder(dialog).pick_files();
    spawn(async move {
        Ok(task
            .await
            .map(|handles| handles.into_iter().map(|handle| path(&handle)).collect()))
    })
}

pub(super) fn alert(
    kind: AlertKind,
    title: String,
    body: String,
) -> Signal<Status<bool, Infallible>> {
    let buttons = match kind {
        AlertKind::Message => MessageButtons::Ok,
        AlertKind::Confirm => MessageButtons::OkCancel,
    };
    let task = AsyncMessageDialog::new()
        .set_title(title)
        .set_description(body)
        .set_buttons(buttons)
        .show();
    spawn(async move {
        Ok(matches!(
            task.await,
            MessageDialogResult::Ok | MessageDialogResult::Yes
        ))
    })
}
