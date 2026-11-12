#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use accesskit::{Action, ActionRequest, TreeId};
use sigil::Host;
use sigil::app::FrameHooks;
use sigil::events::{Event, FocusDirection, request_focus};
use sigil::kernel::{Live, NodeId, announce};
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::widgets::{MenuItem, menu, modal};

const VIEW: (f32, f32) = (480.0, 320.0);

fn settle(host: &mut Host) {
    for _ in 0..3 {
        let _ = host.frame(VIEW, 1.0 / 60.0, FrameHooks::default());
    }
}

fn host(app: impl IntoNode) -> Host {
    let mut host = Host::new(HostMode::Pixel);
    host.mount(app);
    settle(&mut host);
    host
}

fn quiet() -> Host {
    host(column().child(text("Second host")))
}

fn act(host: &mut Host, action: Action, target: NodeId) {
    host.perform(
        target,
        &ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node: target.to_accesskit(),
            data: None,
        },
    );
}

fn click_focused(host: &mut Host) {
    let target = host.focus().expect("a control has focus");
    act(host, Action::Click, target);
}

fn escape() -> Event {
    Event::key(LogicalKey::Named(Key::Escape), KeyPhase::Down)
}

fn speaker(on_click: impl Fn() + 'static) -> Host {
    host(column().child(button("Act").autofocus().activate(on_click)))
}

#[test]
fn an_announcement_stays_with_the_host_that_made_it() {
    Owner::scope("two_hosts::announce", || {
        let mut one = speaker(|| announce("Hello from one", Live::Polite));
        let mut two = quiet();
        click_focused(&mut one);
        settle(&mut two);
        assert!(!two.accessibility_text().contains("Hello from one"));
        settle(&mut one);
        assert!(one.accessibility_text().contains("Hello from one"));
    });
}

#[test]
fn a_focus_request_stays_with_the_host_that_made_it() {
    Owner::scope("two_hosts::focus", || {
        let wanted = Rc::new(Cell::new(None));
        let request = Rc::clone(&wanted);
        let mut one = host(
            column()
                .child(button("Jump").autofocus().activate(move || {
                    if let Some(id) = request.get() {
                        request_focus(id);
                    }
                }))
                .child(button("Target")),
        );
        let jump = one.focus().expect("Jump has focus");
        one.focus_intent(FocusDirection::Down);
        let target = one.focus().expect("Target has focus");
        assert_ne!(jump, target);
        wanted.set(Some(target));
        act(&mut one, Action::Focus, jump);
        act(&mut one, Action::Click, jump);
        let mut two = quiet();
        settle(&mut two);
        settle(&mut one);
        assert_eq!(one.focus(), Some(target));
    });
}

#[test]
fn a_pending_theme_stays_with_the_host_that_set_it() {
    Owner::scope("two_hosts::theme", || {
        let mut one = speaker(|| Theme::dark().apply());
        let mut two = quiet();
        click_focused(&mut one);
        settle(&mut two);
        assert_eq!(two.theme(), &Theme::default());
        settle(&mut one);
        assert_eq!(one.theme(), &Theme::dark());
    });
}

#[test]
fn a_press_in_one_host_does_not_close_a_menu_in_another() {
    Owner::scope("two_hosts::light", || {
        let open = signal(true);
        let mut one = host(column().child(menu(open).item(MenuItem::new("Copy"))));
        let mut two = quiet();
        two.route(Event::pointer_down(Point::new(4.0, 8.0)));
        assert!(open.peek());
        one.route(Event::pointer_down(Point::new(470.0, 310.0)));
        assert!(!open.peek());
    });
}

#[test]
fn escape_in_one_host_closes_only_its_own_modal() {
    Owner::scope("two_hosts::escape", || {
        let show_two = signal(true);
        let mut two = host(modal(show_two).title("Two"));
        let show_one = signal(true);
        let _one = host(modal(show_one).title("One"));
        two.route(escape());
        assert!(!show_two.peek());
        assert!(show_one.peek());
    });
}

#[cfg(all(feature = "dialog", not(feature = "native-dialog")))]
#[test]
fn a_dialog_opens_in_the_host_that_asked_for_it() {
    Owner::scope("two_hosts::dialog", || {
        let mut one = speaker(|| {
            let _status = sigil::app::dialog::message("Prompt one", "Body");
        });
        let mut two = quiet();
        click_focused(&mut one);
        settle(&mut two);
        assert!(two.emit(VIEW).find_text("Prompt one").is_none());
        settle(&mut one);
        assert!(one.emit(VIEW).find_text("Prompt one").is_some());
    });
}

#[test]
fn a_host_made_inside_another_does_not_take_its_announcement() {
    Owner::scope("two_hosts::nested", || {
        let mut one = speaker(|| {
            announce("from one", Live::Polite);
            drop(quiet());
        });
        click_focused(&mut one);
        settle(&mut one);
        assert!(one.accessibility_text().contains("from one"));
    });
}

fn loads(label: &'static str) -> impl IntoNode {
    let status = sigil::tasks::spawn(async move { Ok::<_, ()>(label) });
    text(move || match status.get() {
        sigil::tasks::Status::Ok(label) => format!("{label} loaded"),
        _ => String::from("waiting"),
    })
}

#[test]
fn two_harnesses_each_get_their_own_task_results() {
    let mut first = sigil_kittest::Harness::new(|| loads("first"));
    let mut second = sigil_kittest::Harness::new(|| loads("second"));
    first.expect_text("first loaded");
    second.expect_text("second loaded");
}

#[test]
fn a_task_run_while_another_harness_is_current_reports_to_its_own() {
    let slot = Rc::new(Cell::new(None));
    let export = Rc::clone(&slot);
    let mut first = sigil_kittest::Harness::new(move || {
        let job = sigil::tasks::task(|| async { Ok::<_, ()>("first") });
        export.set(Some(job));
        text(move || match job.status() {
            sigil::tasks::Status::Ok(label) => format!("{label} reloaded"),
            _ => String::from("idle"),
        })
    });
    first.expect_text("idle");
    let mut second = sigil_kittest::Harness::new(|| loads("second"));
    slot.get().expect("task").run();
    first.expect_text("first reloaded");
    second.expect_text("second loaded");
}
