#[path = "../examples/story-07-component.rs"]
#[allow(dead_code)]
mod component;
#[path = "../examples/story-09-custom-widget.rs"]
#[allow(dead_code, unreachable_pub)]
mod custom_widget;
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

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use accesskit::{Action, ActionRequest, TreeId};
use sigil::Host;
use sigil::app::{FrameHooks, HostMode};
use sigil::events::Event;
use sigil::kernel::{NodeId, container};
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::scene::SceneCommand;
use sigil::tasks::AsyncRuntime;
use sigil::widgets::{Framed, WidgetFrame};
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
        let submit = ui.get((Role::Button, "Submit"));
        assert!(submit.is_disabled(), "Submit starts disabled");
        ui.type_into("Name", "Ada")
            .type_into("Email", "ada")
            .expect_text("Enter an email");
        assert!(ui.get((Role::Button, "Submit")).is_disabled());
        ui.type_into("Email", "@x.io");
        assert!(
            !ui.text().contains("Enter an email"),
            "{}",
            ui.accessibility_snapshot()
        );
        assert!(!ui.get((Role::Button, "Submit")).is_disabled());
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
            ui.get((Role::Switch, "Dark mode")).is_checked(),
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
fn custom_rating_chains_widget_paint_states_and_field_on_both_hosts() {
    for mut ui in hosts(|| Harness::new(custom_widget::review_form)) {
        ui.expect_text("Score: 3, Quality: 2")
            .focus_next()
            .press(NamedKey::ArrowRight)
            .expect_text("Score: 4, Quality: 2")
            .press(NamedKey::ArrowLeft)
            .expect_text("Score: 3, Quality: 2")
            .click("5 of 5")
            .expect_text("Score: 5, Quality: 2")
            .click("3 of 4")
            .expect_text("Score: 5, Quality: 3");
    }
}

const BASE_REST: Color = Color::rgb(12, 24, 36);
const BASE_HOVER: Color = Color::rgb(48, 60, 72);
const BASE_PRESSED: Color = Color::rgb(72, 84, 96);
const CALLER_HOVER: Color = Color::rgb(140, 40, 40);
const CALLER_REST: Color = Color::rgb(30, 140, 50);

#[derive(Debug)]
struct PanelButton {
    frame: WidgetFrame,
    hits: Rc<Cell<u32>>,
}

impl PanelButton {
    const fn new(hits: Rc<Cell<u32>>) -> Self {
        Self {
            frame: WidgetFrame::new(),
            hits,
        }
    }
}

impl Framed for PanelButton {
    fn frame_mut(&mut self) -> &mut WidgetFrame {
        &mut self.frame
    }
}

impl IntoNode for PanelButton {
    fn build(self, cx: &mut sigil::node::Cx) -> NodeId {
        let hits = self.hits;
        let root = container()
            .width(60.0)
            .height(48.0)
            .role(Role::Button)
            .label("Panel")
            .background(BASE_REST)
            .hover(|s| s.background(BASE_HOVER))
            .pressed(|s| s.background(BASE_PRESSED))
            .activate(move || hits.set(hits.get() + 1));
        self.frame.finish(root).build(cx)
    }
}

fn panel_rect_colors(ui: &mut Harness) -> Vec<Color> {
    ui.host()
        .emit((480.0, 320.0))
        .commands
        .into_iter()
        .filter_map(|cmd| match cmd {
            SceneCommand::Rect(rect) => Some(rect.color),
            _ => None,
        })
        .collect()
}

#[test]
fn widget_frame_reports_sizes_and_fill_flags() {
    let empty = WidgetFrame::new();
    assert_eq!(empty.width(), None);
    assert_eq!(empty.height(), None);
    assert!(!empty.fills_width());
    assert!(!empty.fills_height());

    let sized = PanelButton::new(Rc::default()).width(96.0).height(36.0);
    assert_eq!(sized.frame.width(), Some(Length::Px(96.0)));
    assert_eq!(sized.frame.height(), Some(Length::Px(36.0)));

    let filled = PanelButton::new(Rc::default()).fill_width().fill_height();
    assert!(filled.frame.fills_width());
    assert!(filled.frame.fills_height());
}

#[test]
fn caller_hover_and_background_override_widget_defaults_on_pixel_host() {
    let hover_hits = Rc::new(Cell::new(0));
    let hover_counter = Rc::clone(&hover_hits);
    let mut hover_ui = Harness::new(move || {
        PanelButton::new(Rc::clone(&hover_counter)).hover(|s| s.background(CALLER_HOVER))
    })
    .pixels(480, 320);
    hover_ui.send([Event::pointer_move(Point::new(12.0, 12.0))]);
    let hover_colors = panel_rect_colors(&mut hover_ui);
    assert!(hover_colors.contains(&CALLER_HOVER));
    assert!(!hover_colors.contains(&BASE_HOVER));
    hover_ui.send([Event::pointer_down(Point::new(12.0, 12.0))]);
    assert!(panel_rect_colors(&mut hover_ui).contains(&BASE_PRESSED));
    hover_ui.send([Event::pointer_up(Point::new(12.0, 12.0))]);
    assert_eq!(hover_hits.get(), 1);

    let bg_hits = Rc::new(Cell::new(0));
    let bg_counter = Rc::clone(&bg_hits);
    let mut bg_ui =
        Harness::new(move || PanelButton::new(Rc::clone(&bg_counter)).background(CALLER_REST))
            .pixels(480, 320);
    bg_ui.send([Event::pointer_move(Point::new(12.0, 12.0))]);
    let bg_colors = panel_rect_colors(&mut bg_ui);
    assert!(bg_colors.contains(&CALLER_REST));
    assert!(!bg_colors.contains(&BASE_HOVER));
    bg_ui.send([
        Event::pointer_down(Point::new(12.0, 12.0)),
        Event::pointer_up(Point::new(12.0, 12.0)),
    ]);
    assert_eq!(bg_hits.get(), 1);
}

#[test]
fn load_runs_inside_an_embedded_host() {
    Owner::scope("embedded-load", || {
        let mut tasks = AsyncRuntime::new().expect("task runtime");
        tasks.install_current();
        let mut host = Host::new(HostMode::Pixel);
        host.mount(loading::loader);
        let load = shown(&mut host, "Load")
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
        host.perform(NodeId::from_accesskit(load), &click);
        assert!(!shown(&mut host, "Loading…").is_empty());
        let deadline = Instant::now() + Duration::from_secs(5);
        while shown(&mut host, "42 items").is_empty() {
            assert!(
                Instant::now() < deadline,
                "the result never reached the host"
            );
            std::thread::sleep(Duration::from_millis(5));
            tasks.pump();
        }
    });
}

fn shown(host: &mut Host, text: &str) -> Vec<(accesskit::NodeId, bool)> {
    drop(host.frame((480.0, 320.0), 1.0 / 60.0, FrameHooks::default()));
    let tree = host.accessibility_tree();
    tree.nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(text) || node.value() == Some(text))
        .map(|(id, node)| (*id, node.supports_action(Action::Click)))
        .collect()
}
