use sigil::prelude::Widget;
use sigil::prelude::{
    AlignItems, Icon, IntoNode, ListRowSlots, NodeBuilder, Paint, Radius, Space, TextContent,
    Token, Tone, column, divider_horizontal, icon, list_row, row, section_label, sidebar, spacer,
    text,
};

use super::data::{RAIL_WIDTH, Section, TEXT_SMALL, TEXT_TITLE, VERSION_LINE, WEIGHT_STRONG};
use super::live::Console;

pub(crate) fn rail(console: Console) -> impl IntoNode {
    sidebar(
        column()
            .fill()
            .padding(Space::Sm)
            .gap(Space::Xs)
            .child(wordmark())
            .child(section_label("Workspace"))
            .child(sections(console))
            .child(spacer())
            .child(divider_horizontal())
            .child(footer(console)),
    )
    .width(RAIL_WIDTH)
    .border_width(1)
    .border_color(Token::Border)
    .radius(Radius::Md)
}

fn wordmark() -> NodeBuilder {
    row()
        .fill_width()
        .gap(Space::Sm)
        .padding_x(Space::Sm)
        .padding_y(Space::Md)
        .align_items(AlignItems::Center)
        .child(icon(Icon::Rocket).tone(Tone::Primary))
        .child(
            text("fleet")
                .nowrap()
                .font_size(TEXT_TITLE, "example layout")
                .font_weight(WEIGHT_STRONG),
        )
}

fn sections(console: Console) -> NodeBuilder {
    column()
        .fill_width()
        .gap(Space::Xs)
        .children_reactive(move || {
            let active = console.section.get();
            Section::ALL
                .iter()
                .map(|&section| nav_row(console, section, section == active))
                .collect::<Vec<_>>()
        })
}

fn nav_row(console: Console, section: Section, active: bool) -> NodeBuilder {
    let slots = ListRowSlots {
        leading: Some(Box::new(icon(section.icon()))),
        badge: section
            .badge()
            .map(|(count, tone)| (TextContent::from(count), tone)),
        ..ListRowSlots::default()
    };
    list_row(active, section.label(), slots).pointer_up(move |_| console.section.set(section))
}

fn footer(console: Console) -> impl IntoNode {
    column()
        .fill_width()
        .gap(Space::Xs)
        .padding_x(Space::Sm)
        .padding_y(Space::Sm)
        .child(
            text(move || format!("{} UTC · last event", console.clock.get()))
                .nowrap()
                .ellipsize()
                .font_size(TEXT_SMALL, "example layout")
                .tabular_nums(true)
                .text_color(Token::OnSurfaceMuted),
        )
        .child(
            text(VERSION_LINE)
                .nowrap()
                .ellipsize()
                .font_size(TEXT_SMALL, "example layout")
                .text_color(Token::OnSurfaceMuted),
        )
}
