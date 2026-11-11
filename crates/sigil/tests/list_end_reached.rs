use std::rc::Rc;

use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil_kittest::Harness;

const PAGE: u32 = 60;
const LAST_PAGE: u32 = 2 * PAGE;

fn rows(len: u32) -> Rc<Vec<u32>> {
    Rc::new((0..len).collect())
}

fn load_more(items: Signal<Rc<Vec<u32>>>, loads: Signal<u32>) {
    loads.update(|count| *count += 1);
    let len = u32::try_from(items.peek().len()).unwrap_or(LAST_PAGE);
    if len < LAST_PAGE {
        items.set(rows(len + PAGE));
    }
}

#[test]
fn scrolling_to_the_end_of_a_list_loads_one_page_per_reach() {
    Owner::scope("list-end-reached", || {
        let items = signal(rows(PAGE));
        let loads = signal(0_u32);
        let mut ui = Harness::new(move || {
            column()
                .child(text(move || format!("Loads: {}", loads.get())))
                .child(
                    virtual_list(items, |_, n: &u32| text(format!("Item {n}")))
                        .viewport_height(320.0)
                        .end_reached(3, move || load_more(items, loads)),
                )
        });
        ui.settle().expect_text("Loads: 0");
        ui.focus_next().press(Key::End).settle();
        ui.expect_text("Loads: 1").expect_text("Item 59");
        ui.press(Key::End).settle();
        ui.expect_text("Loads: 2").expect_text("Item 119");
        ui.press(Key::ArrowUp)
            .press(Key::ArrowDown)
            .press(Key::End)
            .settle();
        ui.expect_text("Loads: 2");
        ui.press(Key::Home).settle().press(Key::End).settle();
        ui.expect_text("Loads: 3");
    });
}

#[test]
fn scrolling_to_the_end_of_a_table_loads_more_rows() {
    Owner::scope("table-end-reached", || {
        let loads = signal(0_u32);
        let state = TableState::new(
            (0..PAGE).collect::<Vec<u32>>(),
            vec![TableColumn::new("n", "Number").cell(|n: &u32| text(format!("Row {n}")))],
        );
        let mut ui = Harness::new(move || {
            let grow = state.clone();
            column()
                .child(text(move || format!("Loads: {}", loads.get())))
                .child(
                    table(state.clone())
                        .viewport_height(320.0)
                        .end_reached(3, move || {
                            loads.update(|count| *count += 1);
                            let len = u32::try_from(grow.rows().peek().len()).unwrap_or(PAGE);
                            grow.replace_rows((0..len + PAGE).collect::<Vec<u32>>());
                        }),
                )
        });
        ui.settle().expect_text("Loads: 0");
        ui.focus_next().press(Key::End).settle();
        ui.expect_text("Loads: 1").expect_text("Row 59");
        ui.press(Key::End).settle();
        ui.expect_text("Loads: 2").expect_text("Row 119");
    });
}
