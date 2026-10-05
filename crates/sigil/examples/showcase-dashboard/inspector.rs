use sigil::prelude::{
    AlignItems, ControlSize, FontFamily, Icon, LayoutBuilderExt, LayoutStyle, NodeBuilder, Paint,
    Radius, TextContent, Token, Tone, WidgetLayoutExt, button, container, derived,
    divider_horizontal, dropdown, field, kv_grid, number_field, progress, scroll, section_label,
    slider, spacer, text, text_field, toggle,
};
use sigil::style::space;
use sigil::widgets::ScrollBuilder;

use super::data::{
    CANARY_STEP, HEALTH_PATH, INSPECTOR_WIDTH, Job, LOG_HEIGHT, LogLine, Region, SURGE_MAX,
    TEXT_SMALL, TEXT_TITLE, WEIGHT_STRONG, duration_label,
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
                .icon(Icon::RefreshCw)
                .outline()
                .size(ControlSize::Compact)
                .activate(move || console.retry(console.focus.peek())),
        )
        .child(
            button("Roll back")
                .icon(Icon::RotateCcw)
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
                .text_color(line.tone.solid())
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

type Field = (&'static str, fn(&Job) -> String);

fn details(console: Console) -> NodeBuilder {
    let fields: [Field; 8] = [
        ("Commit", |job| job.sha.to_owned()),
        ("Author", |job| job.author.to_owned()),
        ("Branch", |job| job.branch.to_owned()),
        ("Region", |job| job.region.label().to_owned()),
        ("Runner", |job| job.runner.to_owned()),
        ("Started", |job| job.started.to_owned()),
        ("Duration", |job| duration_label(job.secs)),
        ("Image size", |job| format!("{} MB", job.image_mb)),
    ];
    kv_grid(
        fields
            .into_iter()
            .map(move |(key, pick)| (key, TextContent::from(read(console, pick)))),
    )
}
