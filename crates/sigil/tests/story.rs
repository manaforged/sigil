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
#[path = "../examples/story-10-layer2-parity.rs"]
#[allow(dead_code, unreachable_pub)]
mod story_10;
#[path = "../examples/story-11-canvas-chart.rs"]
#[allow(dead_code, unreachable_pub)]
mod story_11;
#[path = "../examples/story-12-events-dnd.rs"]
#[allow(dead_code, unreachable_pub)]
mod story_12;
#[path = "../examples/story-13-animation.rs"]
#[allow(dead_code, unreachable_pub)]
mod story_13;
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
use sigil::events::{Event, Shortcut};
use sigil::kernel::{NodeId, container};
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::scene::{Clip, DrawRect, SceneBuilder, SceneCommand};
use sigil::tasks::AsyncRuntime;
use sigil::terminal::cell_point;
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
        .into_commands()
        .into_iter()
        .filter_map(|cmd| match cmd {
            SceneCommand::Rect(rect) => Some(rect.color()),
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

#[test]
fn layer2_parity_example_app_works_on_both_hosts() {
    let _: fn(&'static str) -> story_10::CustomButtonBuilder = story_10::custom_button;
    let _: fn(Signal<f32>, std::ops::RangeInclusive<f32>) -> story_10::CustomSliderBuilder =
        story_10::custom_slider;
    let _: fn(Signal<String>) -> story_10::CustomTextFieldBuilder = story_10::custom_text_field;
    for mut ui in hosts(|| Harness::new(story_10::app)) {
        ui.click("Save")
            .expect_text("Clicks: 1")
            .click(&(Role::Slider, "Volume"))
            .press(NamedKey::ArrowRight)
            .expect_text("Volume: 55")
            .type_into("Enter name", "Ada")
            .press(NamedKey::Enter)
            .expect_text("Submitted: Ada");
    }
}

#[test]
fn layer2_parity_custom_button_click_disabled_and_pending() {
    for mut ui in hosts(|| {
        Harness::new(|| {
            let clicks = signal(0_u32);
            let disabled = signal(false);
            let pending = signal(false);
            column()
                .padding(Space::Md)
                .gap(Space::Sm)
                .background(Token::Surface)
                .child(
                    story_10::custom_button("Submit")
                        .primary()
                        .disabled(disabled)
                        .pending(pending)
                        .activate(move || clicks.update(|n| *n += 1)),
                )
                .child(
                    story_10::custom_button("Toggle Disabled")
                        .ghost()
                        .activate(move || disabled.update(|flag| *flag = !*flag)),
                )
                .child(
                    story_10::custom_button("Toggle Pending")
                        .activate(move || pending.update(|flag| *flag = !*flag)),
                )
                .child(text(move || format!("Clicks: {}", clicks.get())))
        })
    }) {
        ui.click("Submit").expect_text("Clicks: 1");

        ui.click("Toggle Disabled");
        assert!(ui.get((Role::Button, "Submit")).is_disabled());
        ui.click("Submit").expect_text("Clicks: 1");

        ui.click("Toggle Disabled").click("Toggle Pending");
        assert!(ui.get((Role::Button, "Submit")).is_disabled());
        ui.click("Submit").expect_text("Clicks: 1");

        ui.click("Toggle Pending")
            .click("Submit")
            .expect_text("Clicks: 2");
    }
}

#[test]
fn layer2_parity_custom_slider_keys_and_a11y() {
    for mut ui in hosts(|| {
        Harness::new(|| {
            let value = signal(50.0_f32);
            let committed = signal(50.0_f32);
            column()
                .padding(Space::Md)
                .gap(Space::Sm)
                .background(Token::Surface)
                .child(
                    story_10::custom_slider(value, 0.0..=100.0)
                        .step(5.0)
                        .length(200.0)
                        .label("Level")
                        .change(move |next| committed.set(next)),
                )
                .child(text(move || {
                    format!("Level: {:.0} ({:.0})", value.get(), committed.get())
                }))
        })
    }) {
        ui.click(&(Role::Slider, "Level"))
            .press(NamedKey::ArrowRight)
            .expect_text("Level: 55 (55)")
            .press(NamedKey::ArrowLeft)
            .expect_text("Level: 50 (50)")
            .press(NamedKey::Home)
            .expect_text("Level: 0 (0)")
            .press(NamedKey::End)
            .expect_text("Level: 100 (100)");

        let slider_id = ui.host().focus().expect("slider focused");
        for (action, data, expected) in [
            (
                Action::SetValue,
                Some(accesskit::ActionData::NumericValue(40.0)),
                "Level: 40 (40)",
            ),
            (Action::Increment, None, "Level: 45 (45)"),
        ] {
            ui.host().perform(
                slider_id,
                &ActionRequest {
                    action,
                    target_tree: TreeId::ROOT,
                    target_node: accesskit::NodeId(0),
                    data,
                },
            );
            ui.settle().expect_text(expected);
        }
    }
}

#[test]
fn layer2_parity_custom_slider_drag() {
    let mut pixel = Harness::new(|| {
        let value = signal(20.0_f32);
        let committed = signal(20.0_f32);
        column()
            .padding(Space::Md)
            .gap(Space::Sm)
            .background(Token::Surface)
            .child(
                story_10::custom_slider(value, 0.0..=100.0)
                    .step(5.0)
                    .length(200.0)
                    .label("Level")
                    .change(move |next| committed.set(next)),
            )
            .child(text(move || format!("Committed: {:.0}", committed.get())))
    })
    .pixels(480, 320);

    let bounds = pixel
        .get((Role::Slider, "Level"))
        .bounds()
        .expect("slider bounds");
    let mid_y = bounds.height.mul_add(0.5, bounds.origin.y);
    let start = sigil::geometry::Point::new(bounds.width.mul_add(0.2, bounds.origin.x), mid_y);
    let end = sigil::geometry::Point::new(bounds.width.mul_add(0.8, bounds.origin.x), mid_y);
    pixel.drag(&start, &end).expect_text("Committed: 80");
}

#[test]
fn layer2_parity_custom_text_field_editing_history_and_submit() {
    for mut ui in hosts(|| {
        Harness::new(|| {
            let value = signal(String::new());
            let submitted = signal(String::new());
            column()
                .padding(Space::Md)
                .gap(Space::Sm)
                .background(Token::Surface)
                .child(
                    story_10::custom_text_field(value)
                        .placeholder("Query")
                        .width(220.0)
                        .submit(move |text| submitted.set(text.to_owned())),
                )
                .child(text(move || format!("Submitted: {}", submitted.get())))
        })
    }) {
        ui.type_into("Query", "hello world");
        assert_eq!(
            ui.get((Role::TextInput, "Query")).value(),
            Some("hello world")
        );

        ui.press(NamedKey::Backspace);
        assert_eq!(
            ui.get((Role::TextInput, "Query")).value(),
            Some("hello worl")
        );

        ui.press(sigil::events::Shortcut::new(
            NamedKey::Backspace,
            Modifiers::ALT,
        ));
        assert_eq!(ui.get((Role::TextInput, "Query")).value(), Some("hello "));

        ui.press(sigil::events::Shortcut::ctrl(Key::character("a")))
            .type_text("ada");
        assert_eq!(ui.get((Role::TextInput, "Query")).value(), Some("ada"));

        ui.press(sigil::events::Shortcut::ctrl(Key::character("z")));
        assert_ne!(ui.get((Role::TextInput, "Query")).value(), Some("ada"));

        ui.press(sigil::events::Shortcut::ctrl(Key::character("y")));
        assert_eq!(ui.get((Role::TextInput, "Query")).value(), Some("ada"));

        ui.press(NamedKey::Enter).expect_text("Submitted: ada");

        ui.host().route(
            sigil::events::Event::key_down(Key::character("s"))
                .modifiers(Modifiers::ALT)
                .text(Some("ß".to_owned())),
        );
        ui.settle();
        assert_eq!(ui.get((Role::TextInput, "Query")).value(), Some("adaß"));
    }
}

#[test]
fn layer2_parity_invalid_border_persists_while_focused() {
    let red = Color::rgb(211, 47, 47);
    for custom in [false, true] {
        let mut ui = Harness::new(move || {
            let value = signal(String::from("bad"));
            let invalid = signal(true);
            let root = column().padding(Space::Md).background(Token::Surface);
            if custom {
                root.child(
                    story_10::custom_text_field(value)
                        .placeholder("Email")
                        .invalid(invalid),
                )
            } else {
                root.child(text_field(value).placeholder("Email").invalid(invalid))
            }
        })
        .pixels(480, 120);
        ui.click(&(Role::TextInput, "Email"));
        let focused = ui.host().focus().expect("field focused");
        let scene = ui.host().scene((480.0, 120.0)).expect("scene");
        let stroke_color = scene.commands().iter().find_map(|cmd| match cmd {
            SceneCommand::Rect(rect) if rect.node() == Some(focused) => {
                rect.stroke().map(|s| s.color())
            }
            _ => None,
        });
        assert_eq!(
            stroke_color,
            Some(red),
            "custom={custom}: focused invalid field must emit red border stroke"
        );
    }
}

#[test]
fn canvas_chart_app_updates_samples_on_click() {
    for mut ui in hosts(|| Harness::new(story_11::app)) {
        ui.expect_text("Canvas Chart")
            .expect_text("Samples: 5")
            .click("Add sample")
            .expect_text("Samples: 6");
    }
}

#[test]
fn scene_builder_records_commands_and_iterates() {
    let origin = Point::new(0.0, 0.0);
    let rect = Rect::new(origin, 120.0, 80.0);
    let mut builder = SceneBuilder::new();
    builder.clip(Clip::rounded(rect, 8.0), |scene| {
        scene.transform([[1.0, 0.0, 4.0], [0.0, 1.0, 4.0]], |scene| {
            scene.draw_rect(DrawRect::new(Rect::new(origin, 24.0, 12.0), Color::WHITE).radius(4.0));
        });
    });
    let scene = builder.finish();
    assert!(!scene.is_empty());
    assert_eq!(scene.len(), scene.commands().len());
    assert!(
        (&scene)
            .into_iter()
            .any(|cmd| matches!(cmd, SceneCommand::PushClip(_)))
    );
    assert!(
        scene
            .commands()
            .iter()
            .any(|cmd| matches!(cmd, SceneCommand::PushTransform(_)))
    );
    assert!(
        scene
            .commands()
            .iter()
            .any(|cmd| matches!(cmd, SceneCommand::Rect(_)))
    );
}

#[test]
fn events_dnd_story_app_works_on_both_hosts() {
    for mut ui in hosts(|| Harness::new(story_12::app)) {
        ui.expect_text("Events & Drag-Drop")
            .press(NamedKey::Tab)
            .expect_text("Buffer: \"  \"")
            .expect_text("Parent tabs: 1");
        assert!(ui.get((Role::TextInput, "Code buffer")).is_focused());

        ui.click("Stop-only")
            .press(NamedKey::Tab)
            .expect_text("Parent tabs: 1");
        assert!(ui.get((Role::Button, "After stop")).is_focused());

        ui.click("Anchor menu")
            .click("Insert snippet")
            .expect_text("Menu: Inserted snippet");
    }

    let mut pixel = Harness::new(story_12::app).pixels(480, 400);
    let alpha = pixel
        .get((Role::ListItem, "Alpha"))
        .bounds()
        .expect("Alpha bounds");
    let gamma = pixel
        .get((Role::ListItem, "Gamma"))
        .bounds()
        .expect("Gamma bounds");
    let start = Point::new(
        alpha.origin.x + 12.0,
        alpha.height.mul_add(0.5, alpha.origin.y),
    );
    let end = Point::new(
        gamma.origin.x + 12.0,
        gamma.height.mul_add(0.8, gamma.origin.y),
    );
    pixel
        .send([
            Event::pointer_move(start),
            Event::pointer_down(start),
            Event::pointer_drag(Point::new(end.x, end.y - 2.0)),
            Event::pointer_drag(end),
        ])
        .expect_text("▼ after")
        .send([Event::pointer_up(end)])
        .expect_text("Order: Beta -> Gamma -> Alpha");
}

fn mount_selectable_copy_harness(
    prevent_copy: Signal<bool>,
    prevent_escape: Signal<bool>,
    copied: &std::rc::Rc<std::cell::RefCell<Vec<String>>>,
) -> Harness {
    let copied_w = std::rc::Rc::clone(copied);
    let mut ui = Harness::new(move || {
        let copied_w = std::rc::Rc::clone(&copied_w);
        container()
            .selectable(true)
            .focusable(true)
            .autofocus()
            .key_capture(Shortcut::ctrl(Key::character("c")), move |event| {
                if prevent_copy.peek() {
                    event.prevent_default();
                }
            })
            .key_capture(NamedKey::Escape, move |event| {
                if prevent_escape.peek() {
                    event.prevent_default();
                }
            })
            .selection_copy(move |value, _| copied_w.borrow_mut().push(value.to_owned()))
            .child(text("Hello world"))
    });
    ui.send([
        Event::pointer_move(cell_point(0, 0)),
        Event::pointer_down(cell_point(0, 0)),
        Event::pointer_move(cell_point(2, 0)),
        Event::pointer_move(cell_point(4, 0)),
        Event::pointer_up(cell_point(4, 0)),
    ]);
    copied.borrow_mut().clear();
    ui
}

#[test]
fn prevent_default_cancels_copy_and_escape_without_affecting_stop_propagation() {
    Owner::scope("story-prevent-default-copy-escape", || {
        let prevent_copy = signal(true);
        let prevent_escape = signal(true);
        let copied = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
        let mut ui = mount_selectable_copy_harness(prevent_copy, prevent_escape, &copied);

        ui.press(Shortcut::ctrl(Key::character("c")));
        assert!(copied.borrow().is_empty(), "prevent_default blocks Copy");

        prevent_copy.set(false);
        ui.press(Shortcut::ctrl(Key::character("c")));
        assert_eq!(copied.borrow().as_slice(), ["Hello"]);
        copied.borrow_mut().clear();

        ui.press(NamedKey::Escape);
        ui.press(Shortcut::ctrl(Key::character("c")));
        assert_eq!(
            copied.borrow().as_slice(),
            ["Hello"],
            "prevent_default keeps the selection alive on Escape"
        );
        copied.borrow_mut().clear();

        prevent_escape.set(false);
        ui.press(NamedKey::Escape);
        ui.press(Shortcut::ctrl(Key::character("c")));
        assert!(
            copied.borrow().is_empty(),
            "Escape without prevent_default clears the selection"
        );
    });
}

#[test]
fn animation_story_app_works_on_both_hosts() {
    for mut ui in hosts(|| Harness::new(story_13::app)) {
        ui.expect_text("Animation & Springs")
            .expect_text("Ticker: on")
            .click("Toggle")
            .expect_text("Drawer: 1.00 | Active: false")
            .click("Ticker")
            .expect_text("Ticker: off");
    }

    Owner::scope("story-13-pixel-host", || {
        let mut host = Host::new(HostMode::Pixel);
        host.mount(story_13::app);
        for _ in 0..3 {
            let frame = host.frame((480.0, 320.0), 0.016, FrameHooks::default());
            assert!(frame.layout().is_ok());
        }
        assert!(
            host.accessibility_text().contains("Frames: 3 (0.048s)"),
            "{}",
            host.accessibility_text()
        );
    });
}
