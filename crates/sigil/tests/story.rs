#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

#[path = "../examples/story-07-component.rs"]
#[allow(dead_code)]
mod component;
#[path = "../examples/story-03-form.rs"]
#[allow(dead_code)]
mod form;
#[path = "../examples/story-01-hello.rs"]
#[allow(dead_code)]
mod hello;
#[path = "../examples/story-06-async.rs"]
#[allow(dead_code)]
mod loading;
#[path = "../examples/story-05-nav.rs"]
#[allow(dead_code)]
mod nav;
#[path = "../examples/story-08-test.rs"]
#[allow(dead_code)]
mod testing;
#[path = "../examples/story-04-todo.rs"]
#[allow(dead_code)]
mod todo;

use std::time::{Duration, Instant};

use accesskit::{Action, ActionRequest, TreeId};
use sigil::Host;
use sigil::app::FrameHooks;
use sigil::events::EventRouter;
use sigil::kernel::NodeId;
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil::tasks::AsyncRuntime;
use sigil_kittest::{Harness, Query};

fn hosts(app: fn() -> Harness) -> impl Iterator<Item = Harness> {
    [false, true].into_iter().map(move |pixels| {
        if pixels {
            app().pixels(480, 320)
        } else {
            app()
        }
    })
}

#[test]
fn hello_shows_its_text_on_both_hosts() {
    for mut ui in hosts(|| Harness::new(hello::hello)) {
        ui.expect_text("Hello, world");
    }
}

#[test]
fn counter_counts_both_ways_on_both_hosts() {
    for mut ui in hosts(|| Harness::new(testing::counter::counter)) {
        ui.click("+").expect_text("Count: 1");
        ui.click("-").click("-").expect_text("Count: -1");
    }
}

#[test]
fn each_stepper_owns_its_value_on_both_hosts() {
    for mut ui in hosts(|| Harness::new(component::steppers)) {
        let plus = ui.find_all(&Query::role(Role::Button).name("+"));
        let width_plus = plus.first().expect("the Width stepper has a + button");
        ui.click(width_plus);
        let minus = ui.find_all(&Query::role(Role::Button).name("-"));
        let height_minus = minus.last().expect("the Height stepper has a - button");
        ui.click(height_minus).expect_text("2").expect_text("4");
    }
}

#[test]
fn todo_adds_checks_and_deletes_on_both_hosts() {
    for mut ui in hosts(|| Harness::new(todo::todo)) {
        for item in ["milk", "eggs"] {
            ui.type_into("New item", item).click("Add");
        }
        ui.expect_text("2 left").click("milk").expect_text("1 left");
        let delete = ui.find_all(&Query::role(Role::Button).name("Delete"));
        let eggs_delete = delete.last().expect("eggs has a Delete button");
        ui.click(eggs_delete).expect_text("0 left");
        assert!(
            !ui.text().contains("eggs"),
            "{}",
            ui.accessibility_snapshot()
        );
    }
}

#[test]
fn form_validates_and_thanks_on_both_hosts() {
    for mut ui in hosts(|| Harness::new(form::form)) {
        let submit = ui.get_by_role(Role::Button, "Submit");
        assert!(submit.is_disabled(), "Submit starts disabled");
        ui.type_into("Name", "Ada")
            .type_into("Email", "ada")
            .expect_text("Enter an email");
        assert!(ui.get_by_role(Role::Button, "Submit").is_disabled());
        ui.type_into("Email", "@x.io");
        assert!(
            !ui.text().contains("Enter an email"),
            "{}",
            ui.accessibility_snapshot()
        );
        assert!(!ui.get_by_role(Role::Button, "Submit").is_disabled());
        ui.click("Submit").expect_text("Thanks, Ada");
    }
}

#[test]
fn each_page_keeps_its_state_on_both_hosts() {
    for mut ui in hosts(|| Harness::new(nav::pages)) {
        ui.click("Visit")
            .click("Settings")
            .click("Dark mode")
            .click("Home")
            .expect_text("Visits: 1");
        ui.click("Settings");
        assert_eq!(
            ui.get_by_role(Role::Switch, "Dark mode").is_checked(),
            Some(true),
            "{}",
            ui.accessibility_snapshot()
        );
    }
}

#[test]
fn load_shows_loading_then_the_result_on_both_hosts() {
    for mut ui in hosts(|| Harness::new(loading::loader)) {
        ui.click("Load")
            .expect_text("Loading…")
            .expect_text("42 items");
    }
}

#[test]
fn load_runs_inside_an_embedded_host() {
    Runtime::scope("embedded-load", || {
        let mut tasks = AsyncRuntime::new().expect("task runtime");
        tasks.install_current();
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(loading::loader);
        let load = shown(&mut host, &mut router, "Load")
            .into_iter()
            .find(|(_, clickable)| *clickable)
            .map(|(id, _)| id)
            .expect("a Load button");
        let click = ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: load,
            data: None,
        };
        host.perform(&mut router, NodeId::from_accesskit(load), &click);
        assert!(!shown(&mut host, &mut router, "Loading…").is_empty());
        let deadline = Instant::now() + Duration::from_secs(5);
        while shown(&mut host, &mut router, "42 items").is_empty() {
            assert!(
                Instant::now() < deadline,
                "the result never reached the host"
            );
            std::thread::sleep(Duration::from_millis(5));
            tasks.pump();
        }
    });
}

fn shown(host: &mut Host, router: &mut EventRouter, text: &str) -> Vec<(accesskit::NodeId, bool)> {
    drop(host.frame(router, (480.0, 320.0), 1.0 / 60.0, FrameHooks::default()));
    let tree = host.accessibility_tree(router);
    tree.nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(text) || node.value() == Some(text))
        .map(|(id, node)| (*id, node.supports_action(Action::Click)))
        .collect()
}
