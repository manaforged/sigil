use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use crate::kernel::{Cx, NodeId};
use crate::reactive::Runtime;

use super::picker::{alert_view, picker_view};
use super::{AlertKind, FileDialog, PickKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Answer {
    Cancel,
    Paths(Vec<PathBuf>),
    Ok,
}

pub(crate) enum Prompt {
    Files {
        dialog: FileDialog,
        kind: PickKind,
    },
    Alert {
        kind: AlertKind,
        title: String,
        body: String,
    },
}

type Resolve = Box<dyn FnOnce(Answer)>;

#[derive(Clone)]
pub(crate) struct Finish {
    done: Rc<Cell<bool>>,
    resolve: Rc<RefCell<Option<Resolve>>>,
}

impl Finish {
    pub(crate) fn answer(&self, answer: Answer) {
        let resolve = self.resolve.borrow_mut().take();
        if let Some(resolve) = resolve {
            self.done.set(true);
            resolve(answer);
            Runtime::mark_frame_dirty();
        }
    }
}

struct Request {
    prompt: Prompt,
    resolve: Resolve,
}

pub(crate) struct Mounted {
    node: NodeId,
    done: Rc<Cell<bool>>,
}

thread_local! {
    static QUEUE: RefCell<Vec<Request>> = const { RefCell::new(Vec::new()) };
}

#[cfg(feature = "native-dialog")]
pub(super) fn in_app() -> bool {
    let headless = cfg!(all(unix, not(target_os = "macos")))
        && std::env::var_os("DISPLAY").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none();
    crate::app::is_grid() || headless
}

pub(super) fn push_files(
    dialog: FileDialog,
    kind: PickKind,
    resolve: impl FnOnce(Option<Vec<PathBuf>>) + 'static,
) {
    push(Prompt::Files { dialog, kind }, move |answer| {
        resolve(match answer {
            Answer::Paths(paths) => Some(paths),
            Answer::Cancel | Answer::Ok => None,
        });
    });
}

pub(super) fn push_alert(
    kind: AlertKind,
    title: String,
    body: String,
    resolve: impl FnOnce(bool) + 'static,
) {
    push(Prompt::Alert { kind, title, body }, move |answer| {
        resolve(answer == Answer::Ok);
    });
}

fn push(prompt: Prompt, resolve: impl FnOnce(Answer) + 'static) {
    QUEUE.with(|queue| {
        queue.borrow_mut().push(Request {
            prompt,
            resolve: Box::new(resolve),
        });
    });
    Runtime::mark_frame_dirty();
}

pub(crate) fn sync(cx: &mut Cx, root: Option<NodeId>, mounted: &mut Vec<Mounted>) {
    mounted.retain(|entry| {
        if entry.done.get() {
            cx.remove(entry.node);
            false
        } else {
            true
        }
    });
    let Some(root) = root else {
        return;
    };
    let requests = QUEUE.with(|queue| std::mem::take(&mut *queue.borrow_mut()));
    for request in requests {
        let done = Rc::new(Cell::new(false));
        let finish = Finish {
            done: Rc::clone(&done),
            resolve: Rc::new(RefCell::new(Some(request.resolve))),
        };
        let prompt = request.prompt;
        let node = cx.add_child(
            root,
            crate::kernel::WithCx(move |child: &mut Cx| match prompt {
                Prompt::Files { dialog, kind } => picker_view(child, dialog, kind, finish),
                Prompt::Alert { kind, title, body } => alert_view(child, kind, title, body, finish),
            }),
        );
        mounted.push(Mounted { node, done });
    }
}
