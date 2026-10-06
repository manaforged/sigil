#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use std::rc::Rc;

use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil::widgets::virtual_list_bind;
use sigil_kittest::{Harness, Query};

fn rows(from: u64, to: u64) -> Vec<u64> {
    (from..to).collect()
}

fn top_visible(ui: &mut Harness) -> Option<u64> {
    let mut seen: Vec<(f32, u64)> = ui
        .find_all(&Query::role(Role::ListItem))
        .into_iter()
        .filter_map(|node| {
            let top = node.bounds()?.origin.y;
            let id = node.name()?.strip_prefix("item ")?.parse().ok()?;
            Some((top, id))
        })
        .filter(|(top, _)| *top >= 0.0)
        .collect();
    seen.sort_by(|a, b| a.0.total_cmp(&b.0));
    seen.first().map(|(_, id)| *id)
}

#[test]
fn prepending_items_keeps_the_top_visible_item_in_place() {
    Runtime::scope("virtual-list-prepend", || {
        let items = signal(Rc::new(rows(1_000, 1_400)));
        let scroll = signal(0.0_f32);
        let mut ui = Harness::new(move || {
            column().child(
                virtual_list_bind(items, |_, id: &u64| {
                    container()
                        .role(Role::ListItem)
                        .label(format!("item {id}"))
                        .layout(
                            LayoutStyle::block()
                                .fill_width()
                                .height(if id.is_multiple_of(3) { 72.0 } else { 40.0 }),
                        )
                        .child(text(format!("item {id}")))
                })
                .measured()
                .estimate(48.0)
                .key_by(|_, id: &u64| *id)
                .scroll_bind(scroll)
                .viewport_fill(),
            )
        })
        .pixels(600, 600);
        ui.settle();
        scroll.set(4_000.0);
        ui.settle().settle();
        let before = top_visible(&mut ui).unwrap();
        items.update(|all| {
            let mut next = rows(800, 1_000);
            next.extend(all.iter().copied());
            *all = Rc::new(next);
        });
        ui.settle().settle();
        assert_eq!(top_visible(&mut ui), Some(before));
    });
}
