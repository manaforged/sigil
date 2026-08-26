//! Schedule and tool pages.

use sigil::prelude::*;

use crate::icons;
use crate::state::{Page, Shell};

pub(crate) fn schedules(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill()
                .padding(28.0)
                .gap(16.0)
                .min_size(0.0, 0.0)
                .overflow_y(Overflow::Scroll),
        )
        .class(classes!("bg-gray-1"))
        .child(auto_header(shell))
        .child(field("Name", text_field(shell.auto_name)))
        .child(workspace_row())
        .child(prompt_field(shell))
        .child(schedule_row(shell))
}

fn auto_header(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .align_items(AlignItems::Center)
                .gap(10.0),
        )
        .child(back_btn(shell))
        .child(label("Edit schedule").class(classes!("text-lg font-medium text-gray-12")))
        .child(spacer())
        .child(label("Started Jan 28, 2026").class(classes!("text-xs text-gray-11")))
        .child(label("2 runs").class(classes!("text-xs text-gray-11")))
        .child(dot(Tone::Success))
        .child(label("Active").class(classes!("text-xs text-gray-12")))
}

fn back_btn(shell: Shell) -> NodeBuilder {
    container()
        .role(Role::Button)
        .label("Back")
        .focusable(true)
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(28.0, 28.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .class(classes!("rounded-md"))
        .pointer_down(move |event| {
            shell.page.set(Page::Activity);
            event.stop_propagation();
        })
        .child(icons::chevron_left())
}

fn workspace_row() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(6.0))
        .child(label("Workspaces").class(classes!("text-sm text-gray-12")))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .fill_width()
                        .padding(8.0)
                        .align_items(AlignItems::Center),
                )
                .class(classes!("rounded-md border border-gray-6"))
                .child(badge("shot-scraper ×", Tone::Neutral)),
        )
}

fn prompt_field(shell: Shell) -> NodeBuilder {
    field("Instructions", text_edit(shell.auto_prompt).height(132.0))
}

fn schedule_row(shell: Shell) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(8.0))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .fill_width()
                        .align_items(AlignItems::Center)
                        .gap(8.0),
                )
                .child(label("Schedule").class(classes!("text-sm text-gray-12")))
                .child(spacer())
                .child(daily_toggle(shell)),
        )
        .child(time_field())
        .child(day_pills(shell))
}

fn daily_toggle(shell: Shell) -> NodeBuilder {
    let daily = derived(move || shell.auto_daily.get());
    let interval = derived(move || !shell.auto_daily.get());
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(4.0)
                .align_items(AlignItems::Center),
        )
        .child(mode_pill(shell, true, "Daily", daily))
        .child(mode_pill(shell, false, "Interval", interval))
}

fn mode_pill(shell: Shell, daily: bool, title: &'static str, on: Signal<bool>) -> NodeBuilder {
    container()
        .role(Role::Button)
        .label(title)
        .focusable(true)
        .layout(
            LayoutStyle::flex_row()
                .height(24.0)
                .padding_x(10.0)
                .align_items(AlignItems::Center),
        )
        .class(classes!("rounded-full"))
        .class_when(on, "bg-gray-4")
        .pointer_down(move |event| {
            shell.auto_daily.set(daily);
            event.stop_propagation();
        })
        .child(label(title).class(classes!("text-xs text-gray-12")))
}

fn time_field() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding_x(10.0)
                .padding_y(8.0)
                .align_items(AlignItems::Center)
                .gap(8.0)
                .min_size(0.0, 0.0),
        )
        .class(classes!("rounded-md border border-gray-6"))
        .child(label("09:00 AM").class(classes!("text-sm text-gray-12")))
        .child(spacer())
        .child(icons::clock())
}

fn day_pills(shell: Shell) -> NodeBuilder {
    const DAYS: [(u8, &str); 7] = [
        (0, "Mo"),
        (1, "Tu"),
        (2, "We"),
        (3, "Th"),
        (4, "Fr"),
        (5, "Sa"),
        (6, "Su"),
    ];
    let mut row = container().layout(
        LayoutStyle::flex_row()
            .fill_width()
            .gap(4.0)
            .shrink(0.0)
            .justify_content(JustifyContent::End),
    );
    for (day, name) in DAYS {
        let on = derived(move || shell.day_on(day));
        row = row.child(
            container()
                .role(Role::Button)
                .label(name)
                .focusable(true)
                .layout(
                    LayoutStyle::flex_row()
                        .fixed_size(28.0, 28.0)
                        .align_items(AlignItems::Center)
                        .justify_content(JustifyContent::Center),
                )
                .class(classes!("rounded-full border border-gray-6"))
                .class_when(on, "bg-gray-12")
                .pointer_down(move |event| {
                    shell.toggle_day(day);
                    event.stop_propagation();
                })
                .child(
                    label(name)
                        .class(classes!("text-xs text-gray-12"))
                        .class_when(on, "text-gray-1"),
                ),
        );
    }
    row
}

pub(crate) fn tools() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill().padding(28.0).gap(12.0))
        .class(classes!("bg-gray-1"))
        .child(label("Tools").class(classes!("text-lg font-medium text-gray-12")))
        .child(label("Available in this workspace").class(classes!("text-xs text-gray-11")))
        .child(tool_card("Capture", "Save a window or region as an image."))
        .child(tool_card("Annotate", "Mark up captures and project files."))
        .child(tool_card("Export", "Bundle selected notes and assets."))
}

fn tool_card(name: &str, blurb: &str) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding(12.0)
                .gap(4.0),
        )
        .class(classes!("rounded-md border border-gray-6 bg-gray-2"))
        .child(label(name.to_owned()).class(classes!("text-sm font-medium text-gray-12")))
        .child(text(blurb.to_owned()).class(classes!("text-xs text-gray-11")))
}

pub(crate) fn empty_body(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill()
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center)
                .gap(14.0)
                .padding(24.0),
        )
        .child(mark())
        .child(label("Start a work note").class(classes!("text-lg font-medium text-gray-12")))
        .child(label("Local workspace").class(classes!("text-sm text-gray-11")))
        .child(prompt_cards(shell))
}

fn mark() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(40.0, 40.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .class(classes!("rounded-full bg-gray-3"))
        .child(icons::spark())
}

fn prompt_cards(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(10.0)
                .align_items(AlignItems::Start)
                .max_size(640.0, f32::INFINITY),
        )
        .class(classes!("flex-wrap"))
        .child(card(shell, "Review the latest capture."))
        .child(card(shell, "Document the next CLI release."))
        .child(card(shell, "Summarize this week's changes."))
}

fn card(shell: Shell, copy: &'static str) -> NodeBuilder {
    container()
        .role(Role::Button)
        .label(copy)
        .focusable(true)
        .layout(
            LayoutStyle::flex_column()
                .width(200.0)
                .padding(12.0)
                .min_size(0.0, 72.0)
                .overflow(Overflow::Hidden),
        )
        .class(classes!("rounded-md border border-gray-6 bg-gray-2"))
        .pointer_down(move |event| {
            shell.draft.set(copy.to_owned());
            event.stop_propagation();
        })
        .child(text(copy).class(classes!("text-xs text-gray-12")))
}

fn spacer() -> NodeBuilder {
    container().layout(LayoutStyle::block().grow(1.0))
}
