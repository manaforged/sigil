#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::NodeId;
use sigil::prelude::*;

#[derive(Clone, PartialEq)]
struct Todo {
    id: u64,
    text: String,
    done: bool,
}

const FILTERS: [&str; 3] = ["All", "Active", "Done"];

#[derive(Clone, Copy)]
struct Todos {
    items: Signal<Vec<Todo>>,
    next_id: Signal<u64>,
    filter: Signal<usize>,
    cursor: Signal<usize>,
    visible: Signal<Vec<Todo>>,
}

impl Todos {
    fn new() -> Self {
        let seed = [
            ("Read the Sigil terminal guide", true),
            ("Build the todo app", true),
            ("Add filters for active and done", true),
            ("Write the friction log", false),
            ("Review the rc.3 release notes", false),
            ("Reply to the design review thread", false),
            ("Book the team retro", false),
        ];
        let items = signal(
            seed.into_iter()
                .zip(1..)
                .map(|((text, done), id)| Todo {
                    id,
                    text: text.into(),
                    done,
                })
                .collect::<Vec<_>>(),
        );
        let filter = signal(0_usize);
        let cursor = signal(0_usize);
        let visible = memo(move || {
            let f = filter.get();
            items.with(|all| {
                all.iter()
                    .filter(|t| shows(f, t))
                    .cloned()
                    .collect::<Vec<_>>()
            })
        });
        effect(move || {
            filter.with(|_| ());
            cursor.set(0);
        });
        Self {
            items,
            next_id: signal(4),
            filter,
            cursor,
            visible,
        }
    }

    fn add(self, value: &str) {
        let text = value.trim().to_string();
        if !text.is_empty() {
            let id = self.next_id.peek();
            self.next_id.set(id + 1);
            self.items.update(|list| {
                list.push(Todo {
                    id,
                    text,
                    done: false,
                });
            });
        }
    }

    fn target(self) -> Option<u64> {
        self.visible
            .with(|rows| rows.get(self.cursor.peek()).map(|t| t.id))
    }

    fn toggle(self) {
        if let Some(id) = self.target() {
            self.items.update(|list| {
                list.iter_mut()
                    .filter(|t| t.id == id)
                    .for_each(|t| t.done = !t.done);
            });
        }
    }

    fn remove(self) {
        if let Some(id) = self.target() {
            self.items.update(|list| list.retain(|t| t.id != id));
            self.step(0);
        }
    }

    fn step(self, delta: isize) {
        let last = self.visible.with(Vec::len).saturating_sub(1);
        self.cursor
            .update(|at| *at = at.saturating_add_signed(delta).min(last));
    }

    fn cycle_filter(self) {
        self.filter.update(|f| *f = (*f + 1) % FILTERS.len());
    }
}

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("todo").terminal(), app)
}

const fn shows(filter: usize, todo: &Todo) -> bool {
    match filter {
        1 => !todo.done,
        2 => todo.done,
        _ => true,
    }
}

fn filter_strip(active: usize) -> String {
    let name = |(index, name): (usize, &&str)| {
        if index == active {
            format!("[{name}]")
        } else {
            format!(" {name} ")
        }
    };
    FILTERS
        .iter()
        .enumerate()
        .map(name)
        .collect::<Vec<_>>()
        .join(" ")
}

fn list(todos: Todos, list_id: Signal<Option<NodeId>>) -> NodeBuilder {
    let cursor = todos.cursor;
    let entries = scroll(
        column()
            .fill_width()
            .gap(Space::None)
            .children_reactive(move || {
                let at = cursor.get();
                todos
                    .visible
                    .get()
                    .into_iter()
                    .enumerate()
                    .map(move |(index, todo)| {
                        let mark = if todo.done { "[x]" } else { "[ ]" };
                        let row = row()
                            .fill_width()
                            .padding_x(cols(1.0))
                            .gap(Space::None)
                            .align_items(AlignItems::Stretch);
                        let row = if index == at {
                            row.background(Token::Accent5)
                        } else {
                            row
                        };
                        row.child(text(format!("{mark} {}", todo.text)).nowrap().ellipsize())
                    })
                    .collect::<Vec<_>>()
            }),
    )
    .fill();
    column()
        .role(Role::List)
        .label("Todo items")
        .focusable(true)
        .fill_width()
        .grow(1.0)
        .min_height(rows(0.0))
        .gap(Space::None)
        .added(move |_, id| list_id.set(Some(id)))
        .key(NamedKey::Enter, move |_| todos.toggle())
        .key(Key::character(" "), move |_| todos.toggle())
        .key(NamedKey::Delete, move |_| todos.remove())
        .key(Key::character("d"), move |_| todos.remove())
        .key(NamedKey::ArrowDown, move |_| todos.step(1))
        .key(NamedKey::ArrowUp, move |_| todos.step(-1))
        .key(Key::character("j"), move |_| todos.step(1))
        .key(Key::character("k"), move |_| todos.step(-1))
        .key(Key::character("f"), move |_| todos.cycle_filter())
        .child(entries)
}

fn status(todos: Todos) -> NodeBuilder {
    let counts = text(move || {
        let (total, done) = todos
            .items
            .with(|all| (all.len(), all.iter().filter(|t| t.done).count()));
        let filter = FILTERS.get(todos.filter.get()).copied().unwrap_or_default();
        format!(
            "{total} items  {} active  {done} done  [{filter}]",
            total - done
        )
    });
    status_bar([
        counts,
        text("Esc list  Spc done  d del  f filter  q quit").nowrap(),
    ])
}

pub(crate) fn app() -> impl IntoNode {
    let todos = Todos::new();
    let draft = signal(String::new());
    let list_id = signal(None::<NodeId>);
    let quit = Shortcut::plain(Key::character("q"));
    column()
        .fill()
        .gap(Space::None)
        .keyboard_shortcut(quit.clone())
        .key(quit, |_| sigil::exit())
        .key_capture(NamedKey::Escape, move |event| {
            if let Some(id) = list_id.peek() {
                id.focus();
                event.stop_propagation();
            }
        })
        .child(
            row()
                .fill_width()
                .gap(cols(2.0))
                .align_items(AlignItems::Stretch)
                .child(text("Todo").nowrap().text_color(Token::Accent))
                .child(text(move || filter_strip(todos.filter.get())).nowrap()),
        )
        .child(
            text_field(draft)
                .placeholder("New item, Enter to add")
                .autofocus()
                .submit(move |value| {
                    todos.add(value);
                    draft.set(String::new());
                }),
        )
        .child(list(todos, list_id))
        .child(status(todos))
}
