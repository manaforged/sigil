use sigil::prelude::{
    AlignItems, Icon, IntoNode, LayoutBuilderExt, LayoutStyle, ListRowSlots, NodeBuilder,
    NodeEventsExt, Paint, Radius, TextContent, Token, Tone, WidgetLayoutExt, container,
    divider_horizontal, icon, list_row_with, section_label, sidebar, spacer, text,
};
use sigil::style::space;
use sigil::widgets::SidebarBuilder;

use super::data::{RAIL_WIDTH, Section, TEXT_SMALL, TEXT_TITLE, VERSION_LINE, WEIGHT_STRONG};
use super::live::Console;

pub(crate) fn rail(console: Console) -> SidebarBuilder {
    sidebar(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fill()
                    .padding(space(2))
                    .gap(space(1)),
            )
            .child(wordmark())
            .child(section_label("Workspace"))
            .child(sections(console))
            .child(spacer())
            .child(divider_horizontal())
            .child(footer(console)),
    )
    .width(RAIL_WIDTH)
    .border_w(1)
    .border_color(Token::Border)
    .rounded(Radius::Md)
}

fn wordmark() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(space(2))
                .padding_x(space(2))
                .padding_y(space(3))
                .align_items(AlignItems::Center),
        )
        .child(icon(Icon::Rocket).tone(Tone::Primary))
        .child(
            text("fleet")
                .nowrap()
                .font_size(TEXT_TITLE)
                .font_weight(WEIGHT_STRONG),
        )
}

fn sections(console: Console) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(space(1)))
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
    list_row_with(active, section.label(), slots).pointer_up(move |_| console.section.set(section))
}

fn footer(console: Console) -> impl IntoNode {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .gap(space(1))
                .padding_x(space(2))
                .padding_y(space(2)),
        )
        .child(
            text(move || format!("{} UTC · last event", console.clock.get()))
                .nowrap()
                .ellipsize()
                .font_size(TEXT_SMALL)
                .tabular_nums(true)
                .text_color(Token::OnSurfaceMuted),
        )
        .child(
            text(VERSION_LINE)
                .nowrap()
                .ellipsize()
                .font_size(TEXT_SMALL)
                .text_color(Token::OnSurfaceMuted),
        )
}
