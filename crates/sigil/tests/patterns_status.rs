use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::widgets::{badge, breadcrumbs, pagination, skeleton, stepper};
use sigil_kittest::{Harness, Query};

#[test]
fn a_badge_shows_a_count_and_hides_at_zero() {
    Owner::scope("badge", || {
        let unread = signal(0_usize);
        let mut ui = Harness::new(move || {
            column()
                .gap(Space::Sm)
                .child(button("Receive").activate(move || unread.update(|n| *n += 1)))
                .child(
                    row()
                        .gap(Space::Sm)
                        .child(text("Inbox"))
                        .child(badge(unread, Tone::Error)),
                )
        });
        ui.settle();
        assert!(
            ui.find_all(&Query::role(Role::Status)).is_empty(),
            "no badge at zero"
        );
        ui.click("Receive")
            .click("Receive")
            .settle()
            .expect_text("2");
        assert_eq!(ui.find_all(&Query::role(Role::Status).name("2")).len(), 1);
        unread.set(120);
        ui.settle().expect_text("99+");
    });
}

#[test]
fn a_skeleton_reads_as_loading_until_the_content_arrives() {
    Owner::scope("skeleton", || {
        let loading = signal(true);
        let mut ui = Harness::new(move || {
            column()
                .gap(Space::Sm)
                .child(button("Load").activate(move || loading.set(false)))
                .child(show(loading, || skeleton(3), || text("Report ready")))
        });
        ui.settle();
        assert_eq!(
            ui.find_all(&Query::role(Role::Status).name("Loading"))
                .len(),
            1
        );
        ui.click("Load").settle().expect_text("Report ready");
        assert!(
            ui.find_all(&Query::role(Role::Status).name("Loading"))
                .is_empty()
        );
    });
}

const PAGE_BUTTONS: [&str; 9] = [
    "Previous page",
    "1",
    "2",
    "3",
    "4",
    "5",
    "…",
    "12",
    "Next page",
];

#[test]
fn pagination_moves_between_pages_with_buttons_and_keys() {
    Owner::scope("pagination", || {
        let page = signal(0_usize);
        let pages = signal(12_usize);
        let mut ui = Harness::new(move || {
            column()
                .gap(Space::Sm)
                .child(text(move || format!("Showing page {}", page.get() + 1)))
                .child(pagination(page, pages))
        })
        .size(60, 8);
        ui.settle().expect_text("Showing page 1");
        let shown: Vec<String> = ui
            .find_all(&Query::role(Role::Button))
            .iter()
            .filter_map(|node| node.name().map(str::to_owned))
            .collect();
        assert_eq!(
            shown, PAGE_BUTTONS,
            "twelve pages collapse around the current one"
        );
        assert!(ui.get((Role::Button, "Previous page")).is_disabled());
        ui.click("Next page").settle().expect_text("Showing page 2");
        ui.click("12").settle().expect_text("Showing page 12");
        assert!(ui.get((Role::Button, "Next page")).is_disabled());
        assert_eq!(ui.get((Role::Button, "12")).is_selected(), Some(true));
        ui.click("Previous page")
            .settle()
            .expect_text("Showing page 11");
        assert_eq!(
            ui.focused().and_then(|node| node.name().map(str::to_owned)),
            Some("Previous page".to_owned())
        );
        ui.press(NamedKey::Enter)
            .settle()
            .expect_text("Showing page 10");
        assert!(ui.get((Role::Button, "…")).is_disabled(), "gap is inert");
    });
}

#[test]
fn breadcrumbs_navigate_to_an_earlier_segment() {
    Owner::scope("breadcrumbs", || {
        let path = signal(vec![
            "Home".to_owned(),
            "Docs".to_owned(),
            "Install".to_owned(),
        ]);
        let mut ui = Harness::new(move || {
            breadcrumbs(path, move |at| {
                path.update(|segments| segments.truncate(at + 1));
            })
        });
        ui.settle().expect_text("Install");
        assert!(
            ui.find_all(&Query::role(Role::Link).name("Install"))
                .is_empty(),
            "current is not a link"
        );
        ui.click("Docs").settle();
        assert_eq!(path.get(), vec!["Home".to_owned(), "Docs".to_owned()]);
        assert!(ui.find_all(&Query::text("Install")).is_empty());
        assert!(
            ui.find_all(&Query::role(Role::Link).name("Docs"))
                .is_empty(),
            "Docs is now current"
        );
        ui.click("Home").settle();
        assert_eq!(path.get(), vec!["Home".to_owned()]);
    });
}

#[test]
fn a_stepper_marks_done_and_current_steps() {
    Owner::scope("stepper", || {
        let step = signal(0_usize);
        let mut ui = Harness::new(move || {
            column()
                .gap(Space::Sm)
                .child(button("Continue").activate(move || step.update(|n| *n += 1)))
                .child(stepper(["Cart", "Shipping", "Pay"], step))
        })
        .size(60, 8);
        ui.settle();
        assert_eq!(ui.get((Role::ListItem, "Cart")).is_selected(), Some(true));
        assert!(!ui.settle().text().contains('✓'), "nothing done yet");
        ui.click("Continue").settle();
        assert_eq!(
            ui.get((Role::ListItem, "Shipping")).is_selected(),
            Some(true)
        );
        assert_eq!(ui.get((Role::ListItem, "Cart")).is_selected(), Some(false));
        assert!(
            ui.text().contains('✓'),
            "Cart shows a check:\n{}",
            ui.text()
        );
    });
}

#[test]
fn pagination_moves_the_page_into_range_when_pages_shrink() {
    Owner::scope("pagination-shrink", || {
        let page = signal(9_usize);
        let pages = signal(10_usize);
        let mut ui = Harness::new(move || column().child(pagination(page, pages))).size(60, 4);
        ui.settle();
        pages.set(3);
        ui.settle();
        assert_eq!(page.peek(), 2);
        let current = ui.get((Role::Button, "3"));
        assert_eq!(
            current.is_selected(),
            Some(true),
            "the last page is not marked current"
        );
    });
}

#[test]
fn a_badge_at_zero_takes_no_gap_in_its_row() {
    let place = |with_badge: bool| {
        let mut ui = Harness::new(move || {
            let mut line = row().gap(Space::Md).child(text("Inbox"));
            if with_badge {
                line = line.child(badge(signal(0_usize), Tone::Error));
            }
            line.child(text("Sent"))
        })
        .size(40, 4);
        ui.settle();
        ui.get("Sent").bounds().expect("bounds").origin.x
    };
    assert!((place(true) - place(false)).abs() < 0.5);
}
