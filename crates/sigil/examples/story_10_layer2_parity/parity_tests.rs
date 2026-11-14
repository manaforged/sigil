use std::cell::RefCell;

use accesskit::{Action, ActionData, ActionRequest, TreeId};
use sigil::clipboard::{Clipboard, Delivery};
use sigil::events::{Event, Shortcut};
use sigil::prelude::*;
use sigil::scene::SceneCommand;
use sigil_kittest::{Harness, Query};

use super::story_10;

#[derive(Clone, Copy, Debug)]
enum Build {
    Builtin,
    Custom,
}

#[derive(Clone, Copy, Debug)]
enum Input {
    Key(NamedKey, Modifiers),
    Chord(&'static str, Modifiers),
    Type(&'static str),
    Tab,
    Click(Role, &'static str),
    Drag(f32, f32),
    Assist(Action, Option<f64>),
    Send(fn() -> Event),
}

#[derive(Clone, Copy, Debug)]
enum Focus {
    Any,
    On(&'static str),
    Nobody,
}

type Row = (Input, Option<&'static str>, Focus);

const PLAIN: Modifiers = Modifiers::empty();
const CTRL: Modifiers = Modifiers::CTRL;
const SHIFT: Modifiers = Modifiers::SHIFT;
const CONTROLS: [Role; 3] = [Role::Button, Role::Slider, Role::TextInput];

fn apply(ui: &mut Harness, input: Input) {
    match input {
        Input::Key(key, mods) => {
            ui.press(Shortcut::new(key, mods));
        }
        Input::Chord(key, mods) => {
            ui.press(Shortcut::new(Key::character(key), mods));
        }
        Input::Type(chars) => {
            ui.type_text(chars);
        }
        Input::Tab => {
            ui.focus_next();
        }
        Input::Click(role, name) => {
            ui.click(&(role, name));
        }
        Input::Drag(from, to) => {
            let track = ui
                .get(Role::Slider)
                .bounds()
                .expect("the slider is laid out");
            let at = |t: f32| {
                Point::new(
                    track.width.mul_add(t, track.origin.x),
                    track.height.mul_add(0.5, track.origin.y),
                )
            };
            ui.drag(&at(from), &at(to));
        }
        Input::Assist(action, number) => {
            let target = ui.host().focus().expect("a focused control");
            let request = ActionRequest {
                action,
                target_tree: TreeId::ROOT,
                target_node: accesskit::NodeId(0),
                data: number.map(ActionData::NumericValue),
            };
            ui.host().perform(target, &request);
            ui.settle();
        }
        Input::Send(event) => {
            ui.send([event()]);
        }
    }
}

fn seen(ui: &mut Harness) -> Vec<String> {
    ui.find_all(&Query::default())
        .iter()
        .filter(|node| node.name().is_some() || node.value().is_some() || node.is_focused())
        .map(|node| {
            let size = node
                .bounds()
                .filter(|_| CONTROLS.contains(&node.role()))
                .map(|rect| format!("{:.0}x{:.0}", rect.width, rect.height));
            format!(
                "{:?} {:?} value={:?} numeric={:?} disabled={} focused={} checked={:?} size={size:?}",
                node.role(),
                node.name(),
                node.value(),
                node.numeric(),
                node.is_disabled(),
                node.is_focused(),
                node.is_checked(),
            )
        })
        .collect()
}

fn run_script(
    app: fn(Build) -> NodeBuilder,
    build: Build,
    pixels: bool,
    script: &[Row],
) -> Vec<Vec<String>> {
    let ui = Harness::new(move || app(build));
    let mut ui = if pixels { ui.pixels(480, 320) } else { ui };
    let mut frames = Vec::with_capacity(script.len());
    for (index, &(input, shows, focus)) in script.iter().enumerate() {
        apply(&mut ui, input);
        let at = format!("{build:?} pixels={pixels} after step {index} {input:?}");
        if let Some(status) = shows {
            let found = ui.query(&Query::text(status)).is_some();
            assert!(
                found,
                "{at}: expected {status:?}\n{}",
                ui.accessibility_snapshot()
            );
        }
        let focused = ui
            .focused()
            .filter(|node| CONTROLS.contains(&node.role()))
            .and_then(|node| node.name().map(str::to_owned));
        match focus {
            Focus::Any => {}
            Focus::On(name) => assert_eq!(focused.as_deref(), Some(name), "{at}: focus"),
            Focus::Nobody => assert!(focused.is_none(), "{at}: focus {focused:?}"),
        }
        frames.push(seen(&mut ui));
    }
    frames
}

fn parity(app: fn(Build) -> NodeBuilder, surfaces: &[bool], script: &[Row]) {
    for &pixels in surfaces {
        let builtin = run_script(app, Build::Builtin, pixels, script);
        let custom = run_script(app, Build::Custom, pixels, script);
        for (index, (expected, actual)) in builtin.iter().zip(&custom).enumerate() {
            assert_eq!(
                expected,
                actual,
                "pixels={pixels}: after step {index} {:?} the custom widget differs from the built-in one",
                script.get(index).map(|row| row.0),
            );
        }
    }
}

fn panel() -> NodeBuilder {
    column()
        .padding(Space::Md)
        .gap(Space::Sm)
        .background(Token::Surface)
}

fn toggled_by(root: NodeBuilder, key: &'static str, flag: Signal<bool>) -> NodeBuilder {
    root.shortcut(Shortcut::ctrl(Key::character(key)), move |_| {
        flag.update(|on| *on = !*on);
    })
}

fn button_app(build: Build) -> NodeBuilder {
    let clicks = signal(0_u32);
    let (disabled, pending) = (signal(false), signal(false));
    let root = toggled_by(toggled_by(panel(), "d", disabled), "p", pending).child(button("Before"));
    let submit = move || clicks.update(|n| *n += 1);
    match build {
        Build::Builtin => root.child(
            button("Submit")
                .primary()
                .disabled(disabled)
                .pending(pending)
                .activate(submit),
        ),
        Build::Custom => root.child(
            story_10::custom_button("Submit")
                .primary()
                .disabled(disabled)
                .pending(pending)
                .activate(submit),
        ),
    }
    .child(text(move || format!("Clicks: {}", clicks.get())))
}

const BUTTON_SCRIPT: [Row; 16] = [
    (Input::Tab, Some("Clicks: 0"), Focus::On("Submit")),
    (
        Input::Key(NamedKey::Enter, PLAIN),
        Some("Clicks: 1"),
        Focus::On("Submit"),
    ),
    (
        Input::Chord(" ", PLAIN),
        Some("Clicks: 2"),
        Focus::On("Submit"),
    ),
    (
        Input::Click(Role::Button, "Submit"),
        Some("Clicks: 3"),
        Focus::On("Submit"),
    ),
    (
        Input::Chord("d", CTRL),
        Some("Clicks: 3"),
        Focus::On("Before"),
    ),
    (
        Input::Click(Role::Button, "Submit"),
        Some("Clicks: 3"),
        Focus::On("Before"),
    ),
    (Input::Tab, Some("Clicks: 3"), Focus::On("Before")),
    (
        Input::Key(NamedKey::Enter, PLAIN),
        Some("Clicks: 3"),
        Focus::On("Before"),
    ),
    (
        Input::Chord("d", CTRL),
        Some("Clicks: 3"),
        Focus::On("Before"),
    ),
    (Input::Tab, Some("Clicks: 3"), Focus::On("Submit")),
    (Input::Chord("p", CTRL), Some("Clicks: 3"), Focus::Any),
    (
        Input::Key(NamedKey::Enter, PLAIN),
        Some("Clicks: 3"),
        Focus::Any,
    ),
    (Input::Chord(" ", PLAIN), Some("Clicks: 3"), Focus::Any),
    (
        Input::Click(Role::Button, "Submit"),
        Some("Clicks: 3"),
        Focus::Any,
    ),
    (Input::Chord("p", CTRL), Some("Clicks: 3"), Focus::Any),
    (
        Input::Click(Role::Button, "Submit"),
        Some("Clicks: 4"),
        Focus::On("Submit"),
    ),
];

#[test]
fn layer2_parity_button_activates_disables_and_pends_like_the_builtin() {
    parity(button_app, &[false, true], &BUTTON_SCRIPT);
}

fn slider_app(build: Build) -> NodeBuilder {
    let value = signal(50.0_f32);
    let (committed, changes, disabled) = (signal(50.0_f32), signal(0_u32), signal(false));
    let change = move |next: f32| {
        committed.set(next);
        changes.update(|n| *n += 1);
    };
    let root = toggled_by(panel(), "d", disabled);
    match build {
        Build::Builtin => root.child(
            slider(value, 0.0..=100.0)
                .step(5.0)
                .length(200.0)
                .label("Level")
                .disabled(disabled)
                .change(change),
        ),
        Build::Custom => root.child(
            story_10::custom_slider(value, 0.0..=100.0)
                .step(5.0)
                .length(200.0)
                .label("Level")
                .disabled(disabled)
                .change(change),
        ),
    }
    .child(text(move || {
        let (now, last) = (value.get(), committed.get());
        format!(
            "value={now:.0} committed={last:.0} changes={}",
            changes.get()
        )
    }))
}

const fn slid(key: NamedKey, shows: &'static str) -> Row {
    (Input::Key(key, PLAIN), Some(shows), Focus::On("Level"))
}

const SLIDER_KEYS: [Row; 16] = [
    slid(NamedKey::ArrowRight, "value=55 committed=55 changes=1"),
    slid(NamedKey::ArrowUp, "value=60 committed=60 changes=2"),
    slid(NamedKey::ArrowLeft, "value=55 committed=55 changes=3"),
    slid(NamedKey::ArrowDown, "value=50 committed=50 changes=4"),
    slid(NamedKey::PageUp, "value=100 committed=100 changes=5"),
    slid(NamedKey::ArrowRight, "value=100 committed=100 changes=6"),
    slid(NamedKey::PageDown, "value=50 committed=50 changes=7"),
    slid(NamedKey::PageDown, "value=0 committed=0 changes=8"),
    slid(NamedKey::ArrowLeft, "value=0 committed=0 changes=9"),
    slid(NamedKey::End, "value=100 committed=100 changes=10"),
    slid(NamedKey::Home, "value=0 committed=0 changes=11"),
    (
        Input::Assist(Action::SetValue, Some(42.0)),
        Some("value=40 committed=40 changes=12"),
        Focus::On("Level"),
    ),
    (
        Input::Assist(Action::Increment, None),
        Some("value=45 committed=45 changes=13"),
        Focus::On("Level"),
    ),
    (
        Input::Chord("d", CTRL),
        Some("value=45 committed=45 changes=13"),
        Focus::Nobody,
    ),
    (
        Input::Key(NamedKey::ArrowRight, PLAIN),
        Some("value=45 committed=45 changes=13"),
        Focus::Nobody,
    ),
    (
        Input::Chord("d", CTRL),
        Some("value=45 committed=45 changes=13"),
        Focus::On("Level"),
    ),
];

const SLIDER_DRAGS: [Row; 5] = [
    (
        Input::Drag(0.2, 0.8),
        Some("value=80 committed=80 changes=1"),
        Focus::On("Level"),
    ),
    (
        Input::Drag(0.5, 1.2),
        Some("value=100 committed=100 changes=2"),
        Focus::Any,
    ),
    (
        Input::Drag(0.5, -0.1),
        Some("value=0 committed=0 changes=3"),
        Focus::Any,
    ),
    (
        Input::Chord("d", CTRL),
        Some("value=0 committed=0 changes=3"),
        Focus::Nobody,
    ),
    (
        Input::Drag(0.2, 0.8),
        Some("value=0 committed=0 changes=3"),
        Focus::Nobody,
    ),
];

#[test]
fn layer2_parity_slider_keys_assists_drags_and_clamps_like_the_builtin() {
    parity(slider_app, &[false, true], &SLIDER_KEYS);
    parity(slider_app, &[true], &SLIDER_DRAGS);
}

#[derive(Debug, Default)]
struct Memory(RefCell<Option<String>>);

impl Clipboard for Memory {
    fn set(&self, text: &str) -> Result<Delivery, sigil::Error> {
        *self.0.borrow_mut() = Some(text.to_owned());
        Ok(Delivery::Confirmed)
    }

    fn get(&self) -> Option<String> {
        self.0.borrow().clone()
    }
}

fn field_app(build: Build) -> NodeBuilder {
    let value = signal(String::new());
    let (changes, submitted) = (signal(0_u32), signal(String::new()));
    let change = move |_: &str| changes.update(|n| *n += 1);
    let submit = move |done: &str| submitted.set(done.to_owned());
    match build {
        Build::Builtin => panel().child(
            text_field(value)
                .placeholder("Query")
                .width(220.0)
                .clipboard(Memory::default())
                .change(change)
                .submit(submit),
        ),
        Build::Custom => panel().child(
            story_10::custom_text_field(value)
                .placeholder("Query")
                .width(220.0)
                .clipboard(Memory::default())
                .change(change)
                .submit(submit),
        ),
    }
    .child(button("Next"))
    .child(text(move || {
        let shown = format!("[{}] changes={}", value.get(), changes.get());
        format!("{shown} submitted=[{}]", submitted.get())
    }))
}

fn alt_s_types_eszett() -> Event {
    Event::key_down(Key::character("s"))
        .modifiers(Modifiers::ALT)
        .text(Some("ß".to_owned()))
}

const fn typed(input: Input, shows: &'static str) -> Row {
    (input, Some(shows), Focus::On("Query"))
}

const FIELD_SCRIPT: [Row; 32] = [
    typed(Input::Type("hello"), "[hello] changes=5 submitted=[]"),
    typed(
        Input::Key(NamedKey::Backspace, PLAIN),
        "[hell] changes=6 submitted=[]",
    ),
    typed(
        Input::Key(NamedKey::Home, PLAIN),
        "[hell] changes=6 submitted=[]",
    ),
    typed(
        Input::Key(NamedKey::Backspace, PLAIN),
        "[hell] changes=6 submitted=[]",
    ),
    typed(
        Input::Key(NamedKey::Delete, PLAIN),
        "[ell] changes=7 submitted=[]",
    ),
    typed(
        Input::Key(NamedKey::End, PLAIN),
        "[ell] changes=7 submitted=[]",
    ),
    typed(
        Input::Key(NamedKey::Delete, PLAIN),
        "[ell] changes=7 submitted=[]",
    ),
    typed(
        Input::Key(NamedKey::ArrowLeft, SHIFT),
        "[ell] changes=7 submitted=[]",
    ),
    typed(
        Input::Key(NamedKey::ArrowLeft, SHIFT),
        "[ell] changes=7 submitted=[]",
    ),
    typed(Input::Type("p"), "[ep] changes=8 submitted=[]"),
    typed(Input::Chord("a", CTRL), "[ep] changes=8 submitted=[]"),
    typed(Input::Chord("c", CTRL), "[ep] changes=8 submitted=[]"),
    typed(
        Input::Key(NamedKey::End, PLAIN),
        "[ep] changes=8 submitted=[]",
    ),
    typed(Input::Chord("v", CTRL), "[epep] changes=9 submitted=[]"),
    typed(Input::Chord("a", CTRL), "[epep] changes=9 submitted=[]"),
    typed(Input::Chord("x", CTRL), "[] changes=10 submitted=[]"),
    typed(Input::Chord("v", CTRL), "[epep] changes=11 submitted=[]"),
    typed(
        Input::Key(NamedKey::ArrowLeft, PLAIN),
        "[epep] changes=11 submitted=[]",
    ),
    typed(
        Input::Key(NamedKey::Home, SHIFT),
        "[epep] changes=11 submitted=[]",
    ),
    typed(Input::Type("Z"), "[Zp] changes=12 submitted=[]"),
    typed(Input::Chord("u", CTRL), "[p] changes=13 submitted=[]"),
    typed(Input::Chord("k", CTRL), "[] changes=14 submitted=[]"),
    typed(Input::Type("ada"), "[ada] changes=17 submitted=[]"),
    typed(
        Input::Key(NamedKey::Backspace, Modifiers::ALT),
        "[] changes=18 submitted=[]",
    ),
    typed(
        Input::Send(alt_s_types_eszett),
        "[ß] changes=19 submitted=[]",
    ),
    (Input::Chord("z", CTRL), None, Focus::On("Query")),
    typed(Input::Chord("y", CTRL), "[ß] changes=21 submitted=[]"),
    typed(
        Input::Key(NamedKey::Enter, SHIFT),
        "[ß] changes=21 submitted=[]",
    ),
    typed(
        Input::Key(NamedKey::Enter, PLAIN),
        "[ß] changes=21 submitted=[ß]",
    ),
    (
        Input::Tab,
        Some("[ß] changes=21 submitted=[ß]"),
        Focus::On("Next"),
    ),
    (
        Input::Type("x"),
        Some("[ß] changes=21 submitted=[ß]"),
        Focus::On("Next"),
    ),
    (
        Input::Key(NamedKey::Backspace, PLAIN),
        Some("[ß] changes=21 submitted=[ß]"),
        Focus::On("Next"),
    ),
];

#[test]
fn layer2_parity_text_field_edits_selects_clips_and_submits_like_the_builtin() {
    parity(field_app, &[false, true], &FIELD_SCRIPT);
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
