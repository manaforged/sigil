use sigil::prelude::{
    AlignItems, ControlSize, Icon, NodeBuilder, Paint, Space, Token, Tone, button, card, chip,
    column, container, dot, dropdown, row, scroll, spacer, text,
};
use sigil::prelude::{IntoNode, Widget};

use super::data::{
    Bucket, ENV_FILTER_LABELS, ENV_FILTERS, GROUP_WIDTH, Service, TEXT_SMALL, WEIGHT_STRONG,
};
use super::live::Console;
use super::pane;

pub(crate) fn panel(console: Console) -> NodeBuilder {
    pane()
        .width(GROUP_WIDTH)
        .fill_height()
        .shrink(0.0)
        .min_width(0.0)
        .min_height(0.0)
        .padding(Space::Md)
        .gap(Space::Md)
        .child(header(console))
        .child(
            dropdown(console.env_filter, ENV_FILTER_LABELS.iter().copied())
                .accessible("Environment"),
        )
        .child(cards(console))
}

fn header(console: Console) -> NodeBuilder {
    row()
        .fill_width()
        .gap(Space::Sm)
        .align_items(AlignItems::Center)
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

fn cards(console: Console) -> impl IntoNode {
    scroll(
        column()
            .fill_width()
            .gap(Space::Sm)
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
    let body = column()
        .fill_width()
        .gap(Space::Sm)
        .child(
            row()
                .fill_width()
                .gap(Space::Sm)
                .align_items(AlignItems::Center)
                .child(
                    text(service.name())
                        .nowrap()
                        .ellipsize()
                        .font_weight(WEIGHT_STRONG),
                ),
        )
        .child(container().child(chip(env.label(), env.tone())))
        .child(
            row()
                .fill_width()
                .gap(Space::Md)
                .align_items(AlignItems::Center)
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
    row()
        .gap(Space::Xs)
        .align_items(AlignItems::Center)
        .child(dot(tone).size(ControlSize::Compact))
        .child(
            text(move || {
                let count = console.tally(Some(service), bucket) + service.history(bucket);
                count.to_string()
            })
            .nowrap()
            .font_size(TEXT_SMALL, "example layout")
            .tabular_nums(true)
            .text_color(Token::OnSurfaceMuted),
        )
}
