use sigil::prelude::*;

use crate::icons;
use crate::state::{Shell, Where};

pub(crate) fn composer(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding_x(24.0)
                .padding_y(12.0)
                .shrink(0.0),
        )
        .child(
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .fill_width()
                        .padding(16.0)
                        .gap(12.0),
                )
                .class(classes!("rounded-lg border border-gray-6 bg-gray-1"))
                .child(draft_field(shell))
                .child(toolbar(shell)),
        )
}

fn draft_field(shell: Shell) -> NodeBuilder {
    let hint = derived(move || {
        if shell.selected.get() == 0 {
            String::from("Write a new work note")
        } else {
            String::from("Add a follow-up")
        }
    });
    container()
        .layout(LayoutStyle::flex_column().fill_width())
        .child(
            text_field(shell.draft)
                .placeholder(hint)
                .submit(move |_| shell.send())
                .class(classes!("border-0 bg-gray-1")),
        )
}

fn toolbar(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .align_items(AlignItems::Center)
                .gap(8.0),
        )
        .child(icons::plus())
        .child(label("Custom Medium").class(classes!("text-sm text-gray-12")))
        .child(icons::chevron_down())
        .child(spacer())
        .child(send_btn(shell))
}

fn send_btn(shell: Shell) -> NodeBuilder {
    container()
        .role(Role::Button)
        .label("Send")
        .focusable(true)
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(28.0, 28.0)
                .shrink(0.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .class(classes!("rounded-full bg-gray-12"))
        .pointer_down(move |event| {
            shell.send();
            event.stop_propagation();
        })
        .child(icons::send())
}

pub(crate) fn status_strip(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(32.0)
                .padding_x(24.0)
                .align_items(AlignItems::Center)
                .gap(8.0)
                .shrink(0.0),
        )
        .class(classes!("bg-gray-1"))
        .child(run_seg(shell))
        .child(spacer())
        .child(icons::branch())
        .child(label("main").class(classes!("text-xs text-gray-11")))
}

fn run_seg(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(26.0)
                .padding(2.0)
                .gap(2.0)
                .align_items(AlignItems::Center),
        )
        .class(classes!("rounded-md bg-gray-3"))
        .child(seg_item(shell, Where::Local, "Local"))
        .child(seg_item(shell, Where::Worktree, "Worktree"))
        .child(seg_item(shell, Where::Cloud, "Cloud"))
}

fn seg_item(shell: Shell, mode: Where, title: &'static str) -> NodeBuilder {
    let on = derived(move || shell.run.get() == mode);
    container()
        .role(Role::Button)
        .label(title)
        .focusable(true)
        .layout(
            LayoutStyle::flex_row()
                .height(22.0)
                .padding_x(10.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .class(classes!("rounded-sm"))
        .class_when(on, "bg-gray-1")
        .pointer_down(move |event| {
            shell.run.set(mode);
            event.stop_propagation();
        })
        .child(
            label(title)
                .class(classes!("text-xs text-gray-11"))
                .class_when(on, "text-gray-12"),
        )
}

fn spacer() -> NodeBuilder {
    container().layout(LayoutStyle::block().grow(1.0))
}
