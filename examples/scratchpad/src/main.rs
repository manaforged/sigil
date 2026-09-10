use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - scratchpad", 640, 320), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let state = ScratchpadState::new();
    cx.add(scratchpad_root(state))
}

#[derive(Clone, Copy)]
struct ScratchpadState {
    current: Signal<String>,
    saved: Signal<String>,
    status_label: Signal<String>,
    char_label: Signal<String>,
}

impl ScratchpadState {
    fn new() -> Self {
        let current = signal(String::new());
        let saved = signal(String::new());
        let status_label = derived(move || {
            if current.get() == saved.get() {
                String::from("\u{2713} saved")
            } else {
                String::from("\u{25CF} unsaved")
            }
        });
        let char_label = derived(move || format!("{} chars", current.get().chars().count()));
        Self {
            current,
            saved,
            status_label,
            char_label,
        }
    }

    fn save(self) {
        let value = self.current.peek();
        self.saved.set(value);
    }

    fn revert(self) {
        let value = self.saved.peek();
        self.current.set(value);
    }

    fn clear(self) {
        self.current.set(String::new());
    }
}

fn scratchpad_root(state: ScratchpadState) -> NodeBuilder {
    container()
        .role(Role::Main)
        .label("Sigil scratchpad")
        .layout(LayoutStyle::flex_column().gap(12.0).padding(24.0).grow(1.0))
        .class("w-full h-full bg-surface")
        .child(header_row(state))
        .child(editor_row(state))
        .child(footer_row(state))
}

fn header_row(state: ScratchpadState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(12.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween),
        )
        .child(label("Scratchpad"))
        .child(label(state.status_label).class("text-on-surface"))
}

fn editor_row(state: ScratchpadState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(8.0).grow(1.0))
        .child(
            text_field(state.current)
                .placeholder("Type something…")
                .submit(move |_| state.save()),
        )
        .child(label(state.char_label).class("text-on-surface"))
}

fn footer_row(state: ScratchpadState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(8.0)
                .align_items(AlignItems::Center),
        )
        .child(button("Save").primary().activate(move || state.save()))
        .child(button("Revert").activate(move || state.revert()))
        .child(button("Clear").activate(move || state.clear()))
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
