use std::path::PathBuf;
use std::rc::Rc;

use crate::events::prelude::{Key, NodeEventsExt};
use crate::events::request_focus;
use crate::kernel::{Cx, IntoNode, NodeId, container, text};
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle};
use crate::layout::rows;
use crate::reactive::{Signal, effect_named, signal};
use crate::widgets::{button, list_row, modal, text_field, virtual_list};

use super::entries::{Entry, EntryKind, list, start_dir};
use super::layer::{Answer, Finish};
use super::{AlertKind, FileDialog, FileFilter, PickKind};

const LIST_ROWS: f32 = 12.0;

#[derive(Clone)]
struct Picker {
    kind: PickKind,
    filters: Rc<[FileFilter]>,
    dir: Signal<PathBuf>,
    entries: Signal<Vec<Entry>>,
    cursor: Signal<usize>,
    name: Signal<String>,
    finish: Finish,
}

impl Picker {
    fn enter(&self, dir: PathBuf) {
        self.entries.set(list(&dir, self.kind, &self.filters));
        self.dir.set(dir);
        self.cursor.set(0);
    }

    fn up(&self) {
        let parent = self.dir.peek().parent().map(PathBuf::from);
        if let Some(parent) = parent {
            self.enter(parent);
        }
    }

    fn current(&self) -> Option<Entry> {
        self.entries.peek().get(self.cursor.peek()).cloned()
    }

    fn activate(&self) {
        let Some(entry) = self.current() else {
            return;
        };
        match (entry.kind, self.kind) {
            (EntryKind::Parent | EntryKind::Dir, _) => self.enter(entry.path),
            (EntryKind::File, PickKind::Save) => self.name.set(entry.name),
            (EntryKind::File, _) => self.finish.answer(Answer::Paths(vec![entry.path])),
        }
    }

    fn confirm(&self) {
        match self.kind {
            PickKind::Save => {
                let name = self.name.peek();
                if !name.trim().is_empty() {
                    let path = self.dir.peek().join(name.trim());
                    self.finish.answer(Answer::Paths(vec![path]));
                }
            }
            PickKind::Folder => {
                let path = self
                    .current()
                    .filter(|entry| entry.kind == EntryKind::Dir)
                    .map_or_else(|| self.dir.peek(), |entry| entry.path);
                self.finish.answer(Answer::Paths(vec![path]));
            }
            PickKind::File | PickKind::Files => self.activate(),
        }
    }
}

const fn titles(kind: PickKind) -> (&'static str, &'static str) {
    match kind {
        PickKind::File => ("Open file", "Open"),
        PickKind::Files => ("Open files", "Open"),
        PickKind::Folder => ("Choose folder", "Choose"),
        PickKind::Save => ("Save file", "Save"),
    }
}

fn cancel_on_close(show: Signal<bool>, finish: Finish) {
    effect_named("sigil::dialog::close", move || {
        if !show.get() {
            finish.answer(Answer::Cancel);
        }
    });
}

pub(super) fn picker_view(
    cx: &mut Cx,
    dialog: FileDialog,
    kind: PickKind,
    finish: Finish,
) -> NodeId {
    let show = signal(true);
    cancel_on_close(show, finish.clone());
    let start = start_dir(dialog.directory.as_deref());
    let filters: Rc<[FileFilter]> = dialog.filters.into();
    let picker = Picker {
        kind,
        entries: signal(list(&start, kind, &filters)),
        filters,
        dir: signal(start),
        cursor: signal(0),
        name: signal(dialog.file_name.unwrap_or_default()),
        finish: finish.clone(),
    };
    let (title, action) = titles(kind);
    let dir = picker.dir;
    let mut body = modal(show)
        .title(title)
        .child(text(move || dir.get().display().to_string()).nowrap())
        .child(entry_list(picker.clone()));
    if kind == PickKind::Save {
        let submit = picker.clone();
        body = body.child(text_field(picker.name).submit(move |_| submit.confirm()));
    }
    let open = picker;
    body.actions(
        button("Cancel").activate(move || finish.answer(Answer::Cancel)),
        button(action).primary().activate(move || open.confirm()),
    )
    .build(cx)
}

fn entry_list(picker: Picker) -> impl IntoNode {
    move |cx: &mut Cx| {
        let cursor = picker.cursor;
        let rows_view = virtual_list(picker.entries, move |index, entry: &Entry| {
            entry_row(cursor, index, entry)
        })
        .row_height(rows(1.0))
        .viewport_height(rows(LIST_ROWS))
        .keyboard(true)
        .focused_row_bind(cursor)
        .follow(cursor);
        let enter = picker.clone();
        let back = picker.clone();
        let id = container()
            .layout(LayoutStyle::flex_column().fill_width())
            .key(Key::Enter, move |event| {
                event.stop_propagation();
                enter.activate();
            })
            .key(Key::Backspace, move |event| {
                event.stop_propagation();
                back.up();
            })
            .child(rows_view)
            .build(cx);
        focus_first_child(cx, id);
        id
    }
}

fn focus_first_child(cx: &Cx, id: NodeId) {
    if let Some(child) = cx.get(id).and_then(|node| node.children.first().copied()) {
        request_focus(child);
    }
}

fn entry_row(cursor: Signal<usize>, index: usize, entry: &Entry) -> crate::kernel::NodeBuilder {
    let title = entry.title();
    container()
        .layout(LayoutStyle::flex_column().fill_width())
        .pointer_down(move |_| cursor.set(index))
        .children_reactive(move || [list_row(cursor.get() == index, title.clone(), "")])
}

pub(super) fn alert_view(
    cx: &mut Cx,
    kind: AlertKind,
    title: String,
    body: String,
    finish: Finish,
) -> NodeId {
    let show = signal(true);
    cancel_on_close(show, finish.clone());
    let ok = finish.clone();
    let shown = modal(show).title(title).child(text(body).nowrap());
    match kind {
        AlertKind::Message => shown.footer(
            button("OK")
                .primary()
                .activate(move || ok.answer(Answer::Ok)),
        ),
        AlertKind::Confirm => shown.actions(
            button("Cancel").activate(move || finish.answer(Answer::Cancel)),
            button("OK")
                .primary()
                .activate(move || ok.answer(Answer::Ok)),
        ),
    }
    .build(cx)
}
