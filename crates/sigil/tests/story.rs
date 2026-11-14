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
#[path = "../examples/story_10_layer2_parity/parity_tests.rs"]
mod parity;
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
use sigil::scene::{
    Clip, CommandScene, DrawRect, PathCommand, PathPaint, SceneBuilder, SceneCommand,
};
use sigil::tasks::AsyncRuntime;
use sigil::terminal::cell_point;
use sigil::widgets::{Framed, WidgetFrame};
use sigil_kittest::{Harness, Query};

use crate::widget_support::assert_approx;

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
    let mut ui = Harness::new(custom_widget::review_form).pixels(480, 320);
    let ratings = ui.find_all(&Query::role(Role::Slider).name("Rating"));
    let score = ratings
        .first()
        .and_then(sigil_kittest::Node::bounds)
        .expect("the Score rating is laid out");
    let first = rect_of(&mut ui, Role::Button, "1 of 5");
    let second = rect_of(&mut ui, Role::Button, "2 of 5");
    assert_approx(first.origin.x - score.origin.x, 12.0);
    assert_approx(second.origin.x - (first.origin.x + first.width), 8.0);
}

fn rect_of(ui: &mut Harness, role: Role, name: &str) -> Rect {
    ui.get((role, name)).bounds().expect("the node is laid out")
}

fn framed_rating() -> impl IntoNode {
    let score = signal(3_u8);
    let hovered = signal(false);
    column()
        .padding(Space::Md)
        .background(Token::Surface)
        .child(
            custom_widget::rating(score)
                .width(240.0)
                .height(64.0)
                .padding(Space::Lg)
                .track_hovered(hovered)
                .key(NamedKey::Home, move |event| {
                    event.stop_propagation();
                    score.set(1);
                }),
        )
        .child(text(move || {
            format!("Score: {}, Hovered: {}", score.get(), hovered.get())
        }))
}

#[test]
fn custom_rating_frame_applies_caller_size_padding_and_hooks() {
    let mut ui = Harness::new(framed_rating).pixels(480, 320);
    let rating = rect_of(&mut ui, Role::Slider, "Rating");
    let first = rect_of(&mut ui, Role::Button, "1 of 5");
    assert_approx(rating.width, 240.0);
    assert_approx(rating.height, 64.0);
    assert_approx(first.origin.x - rating.origin.x, 16.0);
    let inset = Point::new(rating.origin.x + 4.0, rating.origin.y + 4.0);
    ui.expect_text("Score: 3, Hovered: false")
        .hover(&inset)
        .expect_text("Score: 3, Hovered: true")
        .click(&inset)
        .press(NamedKey::Home)
        .expect_text("Score: 1, Hovered: true");
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
            .accessible("Panel")
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
fn caller_hover_and_background_override_widget_defaults_on_pixel_host() {
    let hover_hits = Rc::new(Cell::new(0));
    let hover_counter = Rc::clone(&hover_hits);
    let mut hover_ui = Harness::new(move || {
        PanelButton::new(Rc::clone(&hover_counter))
            .width(96.0)
            .height(36.0)
            .hover(|s| s.background(CALLER_HOVER))
    })
    .pixels(480, 320);
    let panel = rect_of(&mut hover_ui, Role::Button, "Panel");
    assert_approx(panel.width, 96.0);
    assert_approx(panel.height, 36.0);
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

const CHART: (f32, f32) = (320.0, 140.0);

#[derive(Debug)]
struct Chart {
    bars: Vec<Rect>,
    line: Vec<f32>,
    labels: Vec<String>,
}

const fn same_rgb(a: Color, b: Color) -> bool {
    a.r == b.r && a.g == b.g && a.b == b.b
}

fn is_chart_clip(cmd: &SceneCommand) -> bool {
    matches!(cmd, SceneCommand::PushClip(Clip::Rect(rect))
        if (rect.width - CHART.0).abs() < 0.5 && (rect.height - CHART.1).abs() < 0.5)
}

fn chart_commands(scene: &CommandScene) -> Vec<&SceneCommand> {
    let mut depth = 0_i32;
    scene
        .iter()
        .skip_while(|cmd| !is_chart_clip(cmd))
        .take_while(|cmd| {
            match cmd {
                SceneCommand::PushClip(_) => depth += 1,
                SceneCommand::PopClip => depth -= 1,
                _ => {}
            }
            depth > 0
        })
        .collect()
}

fn read_chart(ui: &mut Harness) -> Chart {
    let accent = ui.host().theme().color(Token::Accent);
    let scene = ui.host().scene((480.0, 320.0)).expect("scene");
    let drawn = chart_commands(&scene);
    assert!(
        !drawn.is_empty(),
        "the canvas clips to its 320 x 140 bounds"
    );
    let accent_line = |cmd: &&SceneCommand| match cmd {
        SceneCommand::Path(path) => path
            .stroke()
            .filter(|stroke| matches!(stroke.paint(), PathPaint::Solid(c) if same_rgb(*c, accent)))
            .map(|_| path.outline().to_vec()),
        _ => None,
    };
    Chart {
        bars: drawn
            .iter()
            .filter_map(|cmd| match cmd {
                SceneCommand::Rect(bar) if same_rgb(bar.color(), accent) => Some(bar.rect()),
                _ => None,
            })
            .collect(),
        line: drawn
            .iter()
            .find_map(accent_line)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|step| match step {
                PathCommand::MoveTo(_, y) | PathCommand::LineTo(_, y) => Some(y),
                _ => None,
            })
            .collect(),
        labels: drawn
            .iter()
            .filter_map(|cmd| match cmd {
                SceneCommand::GlyphRun(run) => Some(run.text().to_owned()),
                _ => None,
            })
            .collect(),
    }
}

fn assert_chart_shows(chart: &Chart, samples: &[f32]) {
    assert_eq!(
        chart.bars.len(),
        samples.len(),
        "one bar per sample: {chart:?}"
    );
    let peak = samples.iter().copied().fold(0.0_f32, f32::max);
    let tallest = chart
        .bars
        .iter()
        .map(|bar| bar.height)
        .fold(0.0_f32, f32::max);
    let floor = chart
        .bars
        .first()
        .map_or(0.0, |bar| bar.origin.y + bar.height);
    for (bar, sample) in chart.bars.iter().zip(samples) {
        assert_approx(bar.height / tallest, sample / peak);
        assert_approx(bar.origin.y + bar.height, floor);
    }
    assert!(
        chart
            .bars
            .iter()
            .zip(chart.bars.iter().skip(1))
            .all(|(left, right)| left.origin.x < right.origin.x),
        "bars run left to right in sample order: {chart:?}"
    );
    assert_eq!(
        chart.line.len(),
        samples.len(),
        "one line point per sample: {chart:?}"
    );
    let highest = chart
        .line
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.total_cmp(b.1));
    let largest = samples.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1));
    assert_eq!(highest.map(|(at, _)| at), largest.map(|(at, _)| at));
    let label = format!("Peak {peak:.0}");
    assert!(chart.labels.contains(&label), "{label:?} in {chart:?}");
}

#[test]
fn canvas_chart_draws_one_accent_bar_and_point_per_sample() {
    for mut ui in hosts(|| Harness::new(story_11::app)) {
        ui.expect_text("Canvas Chart")
            .expect_text("Samples: 5")
            .click("Add sample")
            .expect_text("Samples: 6");
    }
    let mut ui = Harness::new(story_11::app).pixels(480, 320);
    let mut samples = vec![18.0_f32, 42.0, 29.0, 56.0, 38.0];
    assert_chart_shows(&read_chart(&mut ui), &samples);
    ui.click("Add sample");
    samples.push(24.0);
    assert_chart_shows(&read_chart(&mut ui), &samples);
}

#[test]
fn scene_builder_records_nested_clip_transform_and_rect_in_order() {
    let origin = Point::new(0.0, 0.0);
    let rect = Rect::new(origin, 120.0, 80.0);
    let mut builder = SceneBuilder::new();
    builder.clip(Clip::rounded(rect, 8.0), |scene| {
        scene.transform([[1.0, 0.0, 4.0], [0.0, 1.0, 4.0]], |scene| {
            scene.draw_rect(DrawRect::new(Rect::new(origin, 24.0, 12.0), Color::WHITE).radius(4.0));
        });
    });
    let scene = builder.finish();
    let recorded: Vec<&str> = scene
        .iter()
        .map(|cmd| match cmd {
            SceneCommand::PushClip(_) => "clip",
            SceneCommand::PushTransform(_) => "transform",
            SceneCommand::Rect(drawn) if drawn.color() == Color::WHITE => "white rect",
            SceneCommand::PopTransform => "end transform",
            SceneCommand::PopClip => "end clip",
            _ => "other",
        })
        .collect();
    assert_eq!(
        recorded,
        [
            "clip",
            "transform",
            "white rect",
            "end transform",
            "end clip"
        ]
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
