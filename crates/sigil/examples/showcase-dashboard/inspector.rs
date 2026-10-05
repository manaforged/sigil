use sigil::prelude::{
    AlignItems, ControlSize, FontFamily, IntoNode, LayoutBuilderExt, LayoutStyle, NodeBuilder,
    Paint, Radius, Token, Tone, WidgetLayoutExt, avatar, button, container, derived,
    divider_horizontal, dropdown, field, kv_row, number_field, progress, scroll, section_label,
    slider, spacer, text, text_field, toggle,
};
use sigil::style::space;
use sigil::widgets::ScrollBuilder;

use super::data::{
    CANARY_STEP, HEALTH_PATH, INSPECTOR_WIDTH, Job, LOG_HEIGHT, LogLine, Region, SURGE_MAX,
    TEXT_SMALL, TEXT_TITLE, WEIGHT_STRONG, duration_label, tone_token,
};
use super::deploys::status_chip;
use super::live::{Console, Rollout, rollout};
use super::pane;

pub(crate) fn panel(console: Console) -> NodeBuilder {
    pane()
        .layout(
            LayoutStyle::flex_column()
                .width(INSPECTOR_WIDTH)
                .shrink(0.0)
                .fill_height()
                .min_size(0.0, 0.0),
        )
        .child(header(console))
        .child(divider_horizontal())
        .child(
            scroll(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .fill_width()
                            .padding(space(3))
                            .gap(space(3)),
                    )
                    .child(section_label("Live log"))
                    .child(log_view(console))
                    .child(section_label("Rollout"))
                    .child(form(rollout()))
                    .child(section_label("Details"))
                    .child(details(console)),
            )
            .fill(),
        )
}

fn read(console: Console, pick: fn(&Job) -> String) -> impl Fn() -> String + 'static {
    move || console.focused().map(pick).unwrap_or_default()
}

fn header(console: Console) -> NodeBuilder {
    let progress_value = derived(move || console.status_of(console.focus.get()).progress());
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding(space(3))
                .gap(space(2)),
        )
        .child(title_row(console))
        .child(
            text(read(console, |job| job.message.to_owned()))
                .wrap()
                .text_color(Token::OnSurfaceMuted),
        )
        .child(
            progress(progress_value, 1.0)
                .tone(Tone::Primary)
                .label("Rollout"),
        )
        .child(actions(console))
}

fn inline_row() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_width()
        .gap(space(2))
        .align_items(AlignItems::Center)
}

fn title_row(console: Console) -> NodeBuilder {
    container()
        .layout(inline_row())
        .child(
            text(read(console, |job| job.sha.to_owned()))
                .nowrap()
                .font_size(TEXT_TITLE)
                .font_weight(WEIGHT_STRONG)
                .font_family(FontFamily::Mono),
        )
        .child(spacer())
        .child(container().children_reactive(move || [status_chip(console, console.focus.get())]))
}

fn actions(console: Console) -> NodeBuilder {
    container()
        .layout(inline_row())
        .child(
            button("Retry")
                .outline()
                .size(ControlSize::Compact)
                .activate(move || console.retry(console.focus.peek())),
        )
        .child(
            button("Roll back")
                .destructive()
                .size(ControlSize::Compact)
                .activate(move || console.roll_back(console.focus.peek())),
        )
}

fn log_view(console: Console) -> ScrollBuilder {
    scroll(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fill_width()
                    .padding(space(2))
                    .gap(space(1)),
            )
            .children_reactive(move || {
                let slot = console.focus.get();
                let lines = console.log.with(|all| {
                    all.iter()
                        .filter(|line| line.slot == slot)
                        .map(|line| log_line(*line))
                        .collect::<Vec<_>>()
                });
                if lines.is_empty() {
                    vec![quiet("No log lines kept for this run.")]
                } else {
                    lines
                }
            }),
    )
    .follow_end(true)
    .height(LOG_HEIGHT)
    .bg(Token::SurfaceInset)
    .rounded(Radius::Md)
}

fn quiet(body: &'static str) -> NodeBuilder {
    container().child(
        text(body)
            .nowrap()
            .font_size(TEXT_SMALL)
            .text_color(Token::OnSurfaceMuted),
    )
}

fn log_line(line: LogLine) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(space(3))
                .align_items(AlignItems::Start),
        )
        .child(
            text(line.time)
                .nowrap()
                .font_size(TEXT_SMALL)
                .font_family(FontFamily::Mono)
                .text_color(Token::OnSurfaceMuted)
                .layout(LayoutStyle::block().shrink(0.0)),
        )
        .child(
            text(line.text)
                .wrap()
                .font_size(TEXT_SMALL)
                .font_family(FontFamily::Mono)
                .text_color(tone_token(line.tone))
                .layout(LayoutStyle::block().grow(1.0).min_width(0.0)),
        )
}

fn form(rollout: Rollout) -> NodeBuilder {
    let regions = Region::ALL
        .iter()
        .map(|region| region.label().to_owned())
        .collect();
    let canary = rollout.canary;
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(space(3)))
        .child(field(
            "Target region",
            dropdown(rollout.region, regions).label("Target region"),
        ))
        .child(field(
            "Canary percent",
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .fill_width()
                        .gap(space(3))
                        .align_items(AlignItems::Center),
                )
                .child(
                    slider(canary, 0.0..=100.0)
                        .step(CANARY_STEP)
                        .label("Canary percent")
                        .grow(1.0),
                )
                .child(
                    text(move || format!("{:.0}%", canary.get()))
                        .nowrap()
                        .tabular_nums(true),
                ),
        ))
        .child(field(
            "Max surge",
            number_field(rollout.surge, 1..=SURGE_MAX),
        ))
        .child(field(
            "Health check path",
            text_field(rollout.health_path).placeholder(HEALTH_PATH),
        ))
        .child(toggle(rollout.auto_rollback).label("Auto-rollback on failed checks"))
        .child(toggle(rollout.notify).label("Notify on failure"))
}

fn details(console: Console) -> impl IntoNode {
    let author = read(console, |job| job.author.to_owned());
    let initials = read(console, |job| {
        job.author
            .chars()
            .take(2)
            .collect::<String>()
            .to_uppercase()
    });
    container()
        .layout(LayoutStyle::flex_column().fill_width())
        .child(kv_row(
            "Commit",
            mono(read(console, |job| job.sha.to_owned())),
        ))
        .child(kv_row(
            "Author",
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .gap(space(2))
                        .align_items(AlignItems::Center),
                )
                .child(avatar(initials))
                .child(text(author).nowrap()),
        ))
        .child(kv_row(
            "Branch",
            mono(read(console, |job| job.branch.to_owned())),
        ))
        .child(kv_row(
            "Region",
            mono(read(console, |job| job.region.label().to_owned())),
        ))
        .child(kv_row(
            "Runner",
            mono(read(console, |job| job.runner.to_owned())),
        ))
        .child(kv_row(
            "Started",
            mono(read(console, |job| job.started.to_owned())),
        ))
        .child(kv_row(
            "Duration",
            mono(read(console, |job| duration_label(job.secs))),
        ))
        .child(kv_row(
            "Image size",
            mono(read(console, |job| format!("{} MB", job.image_mb))),
        ))
}

fn mono(value: impl Fn() -> String + 'static) -> sigil::kernel::TextBuilder {
    text(value)
        .nowrap()
        .ellipsize()
        .font_family(FontFamily::Mono)
}
