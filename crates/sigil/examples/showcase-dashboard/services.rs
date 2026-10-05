use sigil::prelude::{
    AlignItems, ControlSize, Icon, LayoutBuilderExt, LayoutStyle, NodeBuilder, Paint, Token, Tone,
    button, card, chip, container, dot, dropdown, scroll, spacer, text,
};
use sigil::style::space;
use sigil::widgets::ScrollBuilder;

use super::data::{
    Bucket, ENV_FILTER_LABELS, ENV_FILTERS, GROUP_WIDTH, Service, TEXT_SMALL, WEIGHT_STRONG,
};
use super::live::Console;
use super::pane;

pub(crate) fn column(console: Console) -> NodeBuilder {
    let filters = ENV_FILTER_LABELS
        .iter()
        .map(|label| (*label).to_owned())
        .collect();
    pane()
        .layout(
            LayoutStyle::flex_column()
                .width(GROUP_WIDTH)
                .fill_height()
                .shrink(0.0)
                .min_size(0.0, 0.0)
                .padding(space(3))
                .gap(space(3)),
        )
        .child(header(console))
        .child(dropdown(console.env_filter, filters).label("Environment"))
        .child(cards(console))
}

fn header(console: Console) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(space(2))
                .align_items(AlignItems::Center),
        )
        .child(text("Services").nowrap().font_weight(WEIGHT_STRONG))
        .child(spacer())
        .child(
            button("New")
                .icon(Icon::Plus)
                .ghost()
                .size(ControlSize::Compact)
                .activate(move || console.toasts.info("New service draft opened")),
        )
}

fn cards(console: Console) -> ScrollBuilder {
    scroll(
        container()
            .layout(LayoutStyle::flex_column().fill_width().gap(space(2)))
            .children_reactive(move || {
                let filter = ENV_FILTERS.get(console.env_filter.get()).copied().flatten();
                Service::ALL
                    .iter()
                    .filter(|service| filter.is_none_or(|env| service.env() == env))
                    .map(|&service| service_card(console, service))
                    .collect::<Vec<_>>()
            }),
    )
    .fill()
}

fn service_card(console: Console, service: Service) -> NodeBuilder {
    let env = service.env();
    let body = container()
        .layout(LayoutStyle::flex_column().fill_width().gap(space(2)))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .fill_width()
                        .gap(space(2))
                        .align_items(AlignItems::Center),
                )
                .child(
                    text(service.name())
                        .nowrap()
                        .ellipsize()
                        .font_weight(WEIGHT_STRONG),
                ),
        )
        .child(container().child(chip(env.label(), env.tone())))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .fill_width()
                        .gap(space(3))
                        .align_items(AlignItems::Center),
                )
                .child(counter(console, service, Bucket::Running, Tone::Info))
                .child(counter(console, service, Bucket::Queued, Tone::Neutral))
                .child(counter(console, service, Bucket::Succeeded, Tone::Success))
                .child(counter(console, service, Bucket::Failed, Tone::Error)),
        );
    card(body)
        .selected(move || console.service.get() == service)
        .activate(move || console.service.set(service))
        .into()
}

fn counter(console: Console, service: Service, bucket: Bucket, tone: Tone) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(space(1))
                .align_items(AlignItems::Center),
        )
        .child(dot(tone).size(ControlSize::Compact))
        .child(
            text(move || {
                let count = console.tally(Some(service), bucket) + service.history(bucket);
                count.to_string()
            })
            .nowrap()
            .font_size(TEXT_SMALL)
            .tabular_nums(true)
            .text_color(Token::OnSurfaceMuted),
        )
}
