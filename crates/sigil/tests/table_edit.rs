use sigil::events::Modifiers;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::widgets::Identifiable;
use sigil_kittest::{Harness, Node, Query};

#[derive(Clone)]
struct Person {
    name: String,
    age: u32,
    team: &'static str,
}

fn person(name: &str, age: u32, team: &'static str) -> Person {
    Person {
        name: name.to_owned(),
        age,
        team,
    }
}

fn people() -> TableState<Person> {
    TableState::new(
        vec![person("Ada", 36, "Core"), person("Alan", 41, "Lab")],
        vec![
            TableColumn::new("name", "Name")
                .cell(|p: &Person| text(p.name.clone()))
                .editable(|p| p.name.clone(), |p, name| p.name = name),
            TableColumn::new("team", "Team").cell(|p: &Person| text(p.team)),
            TableColumn::new("age", "Age")
                .cell(|p: &Person| text(p.age.to_string()))
                .editable(
                    |p| p.age.to_string(),
                    |p, age| {
                        if let Ok(age) = age.trim().parse() {
                            p.age = age;
                        }
                    },
                ),
        ],
    )
}

fn name_at(state: &TableState<Person>, index: usize) -> Option<String> {
    state.rows().peek().get(index).map(|p| p.name.clone())
}

fn open(state: &TableState<Person>) -> Harness {
    let shown = state.clone();
    let mut ui = Harness::new(move || table(shown.clone()).viewport_height(240.0));
    ui.settle();
    ui
}

fn editor(ui: &mut Harness) -> Node {
    ui.settle();
    ui.focused()
        .filter(|node| node.role() == Role::TextInput)
        .expect("a cell editor has focus")
}

fn double_click(ui: &mut Harness, label: &str) {
    let cell = ui.get(label);
    ui.click(&cell).click(&cell).settle();
}

fn erase(ui: &mut Harness, chars: usize) {
    for _ in 0..chars {
        ui.press(NamedKey::Backspace);
    }
}

fn editing(ui: &mut Harness) -> bool {
    ui.settle();
    ui.query(&Query::role(Role::TextInput)).is_some()
}

#[test]
fn double_click_edits_a_cell_and_enter_commits_it() {
    Owner::scope("table-edit-commit", || {
        let state = people();
        let mut ui = open(&state);
        double_click(&mut ui, "Ada");
        assert_eq!(editor(&mut ui).name(), Some("Name"));
        erase(&mut ui, 3);
        ui.type_text("Grace").press(NamedKey::Enter).settle();
        ui.expect_text("Grace");
        assert!(!editing(&mut ui), "Enter leaves the editor open");
        assert_eq!(name_at(&state, 0).as_deref(), Some("Grace"));
    });
}

#[test]
fn escape_cancels_an_edit_and_returns_focus_to_the_row() {
    Owner::scope("table-edit-cancel", || {
        let state = people();
        let mut ui = open(&state);
        double_click(&mut ui, "Alan");
        editor(&mut ui);
        ui.type_text("X").press(NamedKey::Escape).settle();
        assert!(!editing(&mut ui), "Escape leaves the editor open");
        ui.expect_text("Alan");
        assert!(ui.query(&Query::text("AlanX")).is_none());
        assert_eq!(name_at(&state, 1).as_deref(), Some("Alan"));
        let focused = ui.focused().map(|node| node.role());
        assert_eq!(focused, Some(Role::Row));
    });
}

#[test]
fn double_click_on_a_read_only_column_does_not_edit() {
    Owner::scope("table-edit-read-only", || {
        let state = people();
        let mut ui = open(&state);
        double_click(&mut ui, "Core");
        assert!(
            !editing(&mut ui),
            "a column without editable() opened an editor"
        );
    });
}

#[test]
fn enter_starts_an_edit_and_tab_walks_the_editable_cells() {
    Owner::scope("table-edit-tab", || {
        let state = people();
        let mut ui = open(&state);
        double_click(&mut ui, "Ada");
        ui.press(NamedKey::Escape).press(NamedKey::Enter);
        let first = editor(&mut ui);
        assert_eq!((first.name(), first.value()), (Some("Name"), Some("Ada")));
        ui.press(NamedKey::Tab);
        let age = editor(&mut ui);
        assert_eq!((age.name(), age.value()), (Some("Age"), Some("36")));
        ui.press(NamedKey::Tab);
        assert_eq!(editor(&mut ui).value(), Some("Alan"));
        ui.press(Shortcut::new(NamedKey::Tab, Modifiers::SHIFT));
        assert_eq!(editor(&mut ui).value(), Some("36"));
        erase(&mut ui, 2);
        ui.type_text("37").press(NamedKey::Enter).settle();
        ui.expect_text("37");
        assert_eq!(state.rows().peek().first().map(|p| p.age), Some(37));
        assert!(!editing(&mut ui));
    });
}

#[derive(Clone)]
struct Named {
    id: u32,
    name: String,
}

impl Identifiable for Named {
    type Id = u32;

    fn id(&self) -> u32 {
        self.id
    }
}

fn named(id: u32, name: &str) -> Named {
    Named {
        id,
        name: name.to_owned(),
    }
}

#[test]
fn an_edit_follows_its_row_when_the_app_replaces_rows() {
    Owner::scope("table-edit-follow", || {
        let state = TableState::identified(
            vec![named(1, "Ada"), named(2, "Alan"), named(3, "Grace")],
            vec![
                TableColumn::new("name", "Name")
                    .cell(|row: &Named| text(row.name.clone()))
                    .editable(|row| row.name.clone(), |row, name| row.name = name),
            ],
        );
        let shown = state.clone();
        let mut ui = Harness::new(move || table(shown.clone()).viewport_height(240.0));
        ui.settle();
        double_click(&mut ui, "Alan");
        assert_eq!(editor(&mut ui).role(), Role::TextInput);
        state.replace_rows(vec![named(2, "Alan"), named(3, "Grace")]);
        assert_eq!(editor(&mut ui).role(), Role::TextInput);
        erase(&mut ui, 8);
        ui.type_text("Bob").press(NamedKey::Enter).settle();
        let names: Vec<String> = state
            .rows()
            .peek()
            .iter()
            .map(|row| row.name.clone())
            .collect();
        assert_eq!(names, ["Bob", "Grace"]);
    });
}

#[test]
fn clicking_outside_the_table_commits_the_open_edit() {
    Owner::scope("table-edit-blur", || {
        let state = people();
        let shown = state.clone();
        let mut ui = Harness::new(move || {
            column()
                .child(table(shown.clone()).viewport_height(240.0))
                .child(button("Elsewhere"))
        });
        ui.settle();
        double_click(&mut ui, "Ada");
        editor(&mut ui);
        erase(&mut ui, 3);
        ui.type_text("Grace");
        let elsewhere = ui.get((Role::Button, "Elsewhere"));
        ui.click(&elsewhere).settle();
        assert_eq!(
            (editing(&mut ui), name_at(&state, 0).as_deref()),
            (false, Some("Grace"))
        );
    });
}
