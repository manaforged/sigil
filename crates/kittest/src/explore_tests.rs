use sigil::prelude::*;

use crate::{Harness, Step};

#[derive(Default)]
struct Spec {
    text: String,
    past: Vec<String>,
    future: Vec<String>,
}

impl Spec {
    fn edit(&mut self, next: String) {
        self.past.push(std::mem::replace(&mut self.text, next));
        self.future.clear();
    }
}

fn typed(ui: &mut Harness, spec: &mut Spec, text: &str) {
    ui.type_text(text);
    let next = format!("{}{text}", spec.text);
    spec.edit(next);
}

const STEPS: [Step<Spec>; 5] = [
    ("type a", |ui, spec| typed(ui, spec, "a")),
    ("type é", |ui, spec| typed(ui, spec, "é")),
    ("backspace", |ui, spec| {
        ui.press(NamedKey::Backspace);
        let mut next = spec.text.clone();
        if next.pop().is_some() {
            spec.edit(next);
        }
    }),
    ("undo", |ui, spec| {
        ui.press_with(Key::character("_"), Modifiers::CTRL);
        if let Some(previous) = spec.past.pop() {
            spec.future
                .push(std::mem::replace(&mut spec.text, previous));
        }
    }),
    ("redo", |ui, spec| {
        ui.press_with(Key::character("y"), Modifiers::CTRL);
        if let Some(next) = spec.future.pop() {
            spec.past.push(std::mem::replace(&mut spec.text, next));
        }
    }),
];

fn field() -> impl IntoNode {
    text_edit(signal(String::new())).autofocus()
}

fn matches_spec(ui: &mut Harness, spec: &Spec) -> Result<(), String> {
    let value = ui
        .focused()
        .and_then(|node| node.value().map(str::to_owned))
        .unwrap_or_default();
    if value == spec.text {
        Ok(())
    } else {
        Err(format!("field shows {value:?}, spec says {:?}", spec.text))
    }
}

#[test]
fn text_edit_matches_its_spec_for_every_four_step_sequence() {
    Harness::explore(field, &STEPS, 4, matches_spec);
}

#[test]
fn text_edit_matches_its_spec_on_a_long_seeded_walk() {
    Harness::walk(field, &STEPS, 200, 7, matches_spec);
}
