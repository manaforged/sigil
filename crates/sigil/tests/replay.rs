use sigil::layout::{Overflow, Position};
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil_kittest::Harness;

fn pin(child: NodeBuilder) -> NodeBuilder {
    container().fill_width().shrink(0.0).child(child)
}

fn prompt(draft: Signal<String>) -> NodeBuilder {
    row()
        .fill_width()
        .align_items(AlignItems::Start)
        .child(text(">"))
        .child(
            column()
                .grow(1.0)
                .min_width(0.0)
                .overflow_y(Overflow::Hidden)
                .child(text(move || draft.get())),
        )
}

fn splash(draft: Signal<String>, open: Signal<bool>) -> NodeBuilder {
    let menu = container().fill_width().children_reactive(move || {
        open.get().then(|| {
            (0..12).fold(column().gap(Space::None), |menu, at| {
                menu.child(text(format!("command {at}")))
            })
        })
    });
    let prompt = prompt(draft);
    let composer = column()
        .fill_width()
        .shrink(0.0)
        .padding_top(Space::Sm)
        .gap(Space::None)
        .child(menu)
        .child(
            column()
                .fill_width()
                .gap(Space::None)
                .child(text("rule"))
                .child(
                    column()
                        .fill_width()
                        .position(Position::Relative)
                        .child(prompt),
                )
                .child(text("hint")),
        );
    column()
        .fill_width()
        .fill_height()
        .gap(Space::None)
        .child(
            column()
                .fill_width()
                .grow(0.0)
                .shrink(1.0)
                .min_height(0.0)
                .height(rows(30.0))
                .child(text("title")),
        )
        .child(pin(composer))
        .child(container().fill_width().grow(1.0))
        .child(pin(container().child(text("footer"))))
}

#[test]
fn a_menu_opened_after_typing_lays_out_like_a_fresh_screen() {
    Owner::scope("replay-splash", || {
        let (draft, open) = (signal(String::new()), signal(false));
        let mut ui = Harness::new(move || splash(draft, open)).size(100, 30);
        ui.settle();
        draft.set("word ".repeat(60));
        ui.settle();
        draft.set(String::new());
        open.set(true);
        ui.settle();
        let (fresh_draft, fresh_open) = (signal(String::new()), signal(true));
        let fresh = Harness::new(move || splash(fresh_draft, fresh_open))
            .size(100, 30)
            .text();
        assert_eq!(ui.text(), fresh);
    });
}
