use sigil::prelude::*;

use crate::icons;
use crate::state::{Page, Shell, Thread};

const RAIL: f32 = 252.0;

pub(crate) fn rail(shell: Shell) -> NodeBuilder {
    container()
        .role(Role::Navigation)
        .label("Workbench")
        .layout(
            LayoutStyle::flex_column()
                .width(RAIL)
                .fill_height()
                .padding_x(12.0)
                .padding_y(14.0)
                .gap(2.0)
                .shrink(0.0),
        )
        .class(classes!("bg-gray-3"))
        .child(nav_item(shell, Page::Activity, "New note", true))
        .child(nav_item(shell, Page::Schedules, "Schedules", false))
        .child(nav_item(shell, Page::Tools, "Tools", false))
        .child(section_label())
        .child(thread_list(shell))
        .child(spacer())
        .child(account_row())
}

fn nav_item(shell: Shell, page: Page, title: &'static str, new_thread: bool) -> NodeBuilder {
    let on = derived(move || {
        if new_thread {
            shell.page.get() == Page::Activity && shell.selected.get() == 0
        } else {
            shell.page.get() == page
        }
    });
    container()
        .role(Role::Button)
        .label(title)
        .focusable(true)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(30.0)
                .padding_x(8.0)
                .gap(8.0)
                .align_items(AlignItems::Center),
        )
        .class(classes!("rounded-md"))
        .class_when(on, "bg-gray-5")
        .pointer_down(move |event| {
            if new_thread {
                shell.new_thread();
            } else {
                shell.page.set(page);
            }
            event.stop_propagation();
        })
        .child(nav_icon(page))
        .child(label(title).class(classes!("text-sm text-gray-12")))
}

fn nav_icon(page: Page) -> PathNode {
    match page {
        Page::Activity => icons::compose(),
        Page::Schedules => icons::clock(),
        Page::Tools => icons::spark(),
    }
}

fn section_label() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding_y(10.0)
                .padding_x(8.0)
                .gap(6.0)
                .align_items(AlignItems::Center),
        )
        .child(label("Work log").class(classes!("text-xs text-gray-11")))
        .child(spacer())
        .child(icons::archive())
        .child(icons::filter())
}

fn thread_list(shell: Shell) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().grow(1.0))
        .children_keyed(move || {
            let rows = shell.threads.get();
            let mut out = Vec::new();
            let mut last = "";
            for thread in rows {
                if thread.project != last {
                    last = thread.project;
                    out.push((
                        format!("p-{}", thread.project),
                        node(project_header(thread.project)),
                    ));
                }
                let id = thread.id;
                out.push((format!("t-{id}"), node(thread_row(shell, &thread))));
            }
            out
        })
}

fn project_header(name: &str) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(26.0)
                .padding_x(8.0)
                .gap(6.0)
                .align_items(AlignItems::Center),
        )
        .child(icons::folder())
        .child(label(name.to_owned()).class(classes!("text-xs text-gray-12")))
}

fn thread_row(shell: Shell, thread: &Thread) -> NodeBuilder {
    let id = thread.id;
    let on = derived(move || shell.selected.get() == id && shell.page.get() == Page::Activity);
    container()
        .role(Role::Button)
        .label(thread.title.clone())
        .focusable(true)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(28.0)
                .padding_x(18.0)
                .gap(6.0)
                .align_items(AlignItems::Center),
        )
        .class(classes!("rounded-md"))
        .class_when(on, "bg-gray-5")
        .pointer_down(move |event| {
            shell.select(id);
            event.stop_propagation();
        })
        .child(status_mark(thread))
        .child(
            label(thread.title.clone())
                .ellipsize()
                .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 16.0))
                .class(classes!("text-sm text-gray-12")),
        )
        .child(row_meta(thread))
}

fn status_mark(thread: &Thread) -> NodeBuilder {
    if thread.pinned {
        return container()
            .layout(LayoutStyle::block().fixed_size(14.0, 14.0))
            .child(icons::pin());
    }
    if thread.running {
        return container()
            .layout(LayoutStyle::block().fixed_size(10.0, 10.0))
            .class(classes!("rounded-full border border-gray-8"));
    }
    container().layout(LayoutStyle::block().fixed_size(14.0, 14.0))
}

fn row_meta(thread: &Thread) -> NodeBuilder {
    if thread.pinned {
        return container()
            .layout(LayoutStyle::block().fixed_size(14.0, 14.0))
            .child(icons::calendar());
    }
    container().child(label(thread.age).class(classes!("text-xs text-gray-11")))
}

fn spacer() -> NodeBuilder {
    container().layout(LayoutStyle::block().grow(1.0))
}

fn account_row() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(36.0)
                .padding_x(8.0)
                .gap(8.0)
                .align_items(AlignItems::Center),
        )
        .child(icons::avatar())
        .child(label("Personal").class(classes!("text-sm text-gray-12")))
}
