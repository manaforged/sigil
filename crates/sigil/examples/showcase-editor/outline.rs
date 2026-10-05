use sigil::prelude::*;
use sigil::style::space;
use sigil::widgets::ListRowSlots;

use super::facts::{SYMBOLS, Symbol};
use super::state::Workspace;

fn symbol_row(ws: Workspace, symbol: Symbol, selected: bool) -> NodeBuilder {
    list_row_with(
        selected,
        symbol.name,
        ListRowSlots {
            depth: symbol.depth,
            marker: Some(symbol.kind.badge().into()),
            status: Some((symbol.line + 1).to_string().into()),
            ..ListRowSlots::default()
        },
    )
    .pointer_down(move |_| ws.goto(symbol.file, symbol.line, 0))
}

fn current_symbol(symbols: &[Symbol], line: usize) -> Option<usize> {
    symbols.iter().rposition(|symbol| symbol.line <= line)
}

pub(crate) fn outline(ws: Workspace) -> NodeBuilder {
    let list = container()
        .layout(LayoutStyle::flex_column().fill_width())
        .children_reactive(move || {
            let Some(file) = ws.active() else {
                return vec![empty_state("No symbols", "Open a source file.")];
            };
            let symbols: Vec<Symbol> = SYMBOLS
                .iter()
                .copied()
                .filter(|symbol| symbol.file == file)
                .collect();
            if symbols.is_empty() {
                return vec![empty_state("No symbols", "This file has no outline.")];
            }
            let line = ws.buffer(file).caret.get().0;
            let here = current_symbol(&symbols, line);
            symbols
                .into_iter()
                .enumerate()
                .map(|(index, symbol)| symbol_row(ws, symbol, Some(index) == here))
                .collect()
        });
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill()
                .min_size(0.0, 0.0)
                .padding_y(space(1)),
        )
        .bg(Token::SurfaceSubtle)
        .rounded(Radius::Md)
        .child(section_label("OUTLINE"))
        .child(
            container()
                .layout(LayoutStyle::flex_column().grow(1.0).min_size(0.0, 0.0))
                .child(scroll(list).fill()),
        )
}
