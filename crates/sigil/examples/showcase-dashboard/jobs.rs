use sigil::prelude::{
    AlignItems, Icon, LayoutBuilderExt, LayoutStyle, NodeBuilder, NodeStyleExt, Paint, TableState,
    Token, Tone, WidgetLayoutExt, button, chip, container, derived, icon, spacer, stat, text,
    text_field,
};
use sigil::style::space;
use sigil::widgets::StatBuilder;

use super::data::{
    Bucket, DEPLOYS_BEFORE_OPEN, TEXT_SMALL, TEXT_TITLE, WEIGHT_STRONG, duration_label,
};
use super::deploys::{JobRow, job_table};
use super::live::Console;
use super::pane;
use super::seed::{
    P95_DURATION, P95_TREND, SUCCESS_HIGH, SUCCESS_LOW, SUCCESS_RATE, SUCCESS_TREND,
};

pub(crate) fn main_panel(console: Console, state: &TableState<JobRow>) -> NodeBuilder {
    pane()
        .layout(
            LayoutStyle::flex_column()
                .grow(1.0)
                .flex_basis(0.0)
                .fill_height()
                .min_size(0.0, 0.0)
                .padding(space(2))
                .gap(space(3)),
        )
        .child(toolbar(console, state))
        .child(metrics(console))
        .child(search_row(state))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .fill_width()
                        .grow(1.0)
                        .min_size(0.0, 0.0),
                )
                .child(job_table(console, state.clone())),
        )
}

fn toolbar(console: Console, state: &TableState<JobRow>) -> NodeBuilder {
    let selection = state.selection();
    let picked = derived(move || selection.get().len());
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(space(2))
                .align_items(AlignItems::Center),
        )
        .child(
            button("Deploy")
                .primary()
                .activate(move || console.deploy()),
        )
        .child(heading(console))
        .child(spacer())
        .child(
            container()
                .visible_when(move || picked.get() > 0)
                .child(chip(
                    move || format!("{} selected", picked.get()),
                    Tone::Primary,
                )),
        )
        .child(pause_controls(console))
}

fn heading(console: Console) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .min_width(0.0)
                .padding_x(space(2)),
        )
        .child(
            text(move || console.service.get().name().to_owned())
                .nowrap()
                .ellipsize()
                .font_size(TEXT_TITLE)
                .font_weight(WEIGHT_STRONG),
        )
        .child(
            text(move || {
                let env = console.service.get().env().label();
                format!("Deploys · {env}")
            })
            .nowrap()
            .font_size(TEXT_SMALL)
            .text_color(Token::OnSurfaceMuted),
        )
}

fn pause_controls(console: Console) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().gap(space(2)))
        .child(
            button("Pause all")
                .outline()
                .disabled(move || console.paused.get())
                .activate(move || console.set_paused(true)),
        )
        .child(
            button("Resume all")
                .outline()
                .disabled(move || !console.paused.get())
                .activate(move || console.set_paused(false)),
        )
}

fn tile(stat: StatBuilder) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .grow(1.0)
                .flex_basis(0.0)
                .min_width(0.0),
        )
        .child(stat)
}

fn metrics(console: Console) -> NodeBuilder {
    let deploys = move || {
        let landed = console.tally(None, Bucket::Succeeded);
        (DEPLOYS_BEFORE_OPEN + landed).to_string()
    };
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(space(2))
                .align_items(AlignItems::Stretch),
        )
        .child(tile(
            stat("Success rate", SUCCESS_RATE)
                .trend(SUCCESS_TREND.to_vec())
                .range(SUCCESS_LOW, SUCCESS_HIGH)
                .tone(Tone::Success),
        ))
        .child(tile(
            stat("p95 duration", P95_DURATION).delta(P95_TREND, Tone::Success),
        ))
        .child(tile(stat("Deploys today", deploys).caption(move || {
            format!("{} running now", console.tally(None, Bucket::Running))
        })))
        .child(tile(
            stat("Mean duration", move || {
                duration_label(mean_secs(&console.durations.get()))
            })
            .trend(move || console.durations.get())
            .tone(Tone::Info),
        ))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "durations are a few minutes, far inside u32 and f32 precision"
)]
fn mean_secs(minutes: &[f32]) -> u32 {
    let total: f32 = minutes.iter().sum();
    (total * 60.0 / minutes.len().max(1) as f32)
        .round()
        .max(0.0) as u32
}

fn search_row(state: &TableState<JobRow>) -> NodeBuilder {
    let visible = state.visible();
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(space(2))
                .align_items(AlignItems::Center),
        )
        .child(
            text_field(state.quick_filter())
                .leading(icon(Icon::Search))
                .placeholder("Filter by commit, author, branch, or runner")
                .clearable()
                .grow(1.0),
        )
        .child(
            text(move || format!("{} jobs", visible.get().len()))
                .nowrap()
                .font_size(TEXT_SMALL)
                .tabular_nums(true)
                .text_color(Token::OnSurfaceMuted),
        )
}
