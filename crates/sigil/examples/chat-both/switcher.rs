use super::Chat;
use sigil::prelude::*;
use sigil::widgets::ModalBuilder;

#[derive(Clone, Copy)]
pub(super) struct Picker {
    pub(super) open: Signal<bool>,
    pub(super) query: Signal<String>,
    pub(super) cursor: Signal<usize>,
    pub(super) focus: Signal<u64>,
}

fn rank(names: &[String], query: &str) -> Vec<usize> {
    if query.trim().is_empty() {
        return (0..names.len()).collect();
    }
    let mut hits: Vec<(usize, usize)> = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| fuzzy_score(query, name).map(|hit| (hit.score, index)))
        .collect();
    hits.sort_unstable();
    hits.into_iter().map(|(_, index)| index).collect()
}

fn pick(chat: Chat, picker: Picker, channel: usize) {
    chat.active.set(channel);
    picker.open.set(false);
}

pub(super) fn picker_modal(chat: Chat, picker: Picker) -> ModalBuilder {
    let hits = derived(move || {
        let query = picker.query.get();
        chat.names.with(|names| rank(names, &query))
    });
    let step = move |forward: bool| {
        let count = hits.with(Vec::len).max(1);
        let shift = if forward { 1 } else { count - 1 };
        picker.cursor.update(|at| *at = (*at + shift) % count);
    };
    let search = text_field(picker.query)
        .placeholder("Jump to channel")
        .focus_when(picker.focus)
        .change(move |_| picker.cursor.set(0))
        .submit(move |_| {
            if let Some(channel) = hits.with(|h| h.get(picker.cursor.peek()).copied()) {
                pick(chat, picker, channel);
            }
        });
    let results = container()
        .layout(LayoutStyle::flex_column().fill_width())
        .children_reactive(move || {
            let cursor = picker.cursor.get();
            let names = chat.names.get();
            let entries = hits.get().into_iter().enumerate();
            entries
                .filter_map(|(row, channel)| {
                    let name = names.get(channel)?;
                    let item = list_row(row == cursor, format!("# {name}"), "");
                    Some(item.pointer_up(move |_| pick(chat, picker, channel)))
                })
                .collect::<Vec<_>>()
        });
    modal(picker.open)
        .title("Switch channel")
        .dismiss_on_backdrop(true)
        .child(
            container()
                .layout(LayoutStyle::flex_column().fill_width().gap(rows(0.5)))
                .key_capture(Key::ArrowDown, move |_| step(true))
                .key_capture(Key::ArrowUp, move |_| step(false))
                .child(search)
                .child(results),
        )
}
