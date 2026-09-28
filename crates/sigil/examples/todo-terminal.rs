#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

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
        let visible = derived(move || {
            let f = filter.get();
            items.with(|all| {
                all.iter()
                    .filter(|t| shows(f, t))
                    .cloned()
                    .collect::<Vec<_>>()
            })
        });
        effect(move || {
            filter.track();
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
    let entries = scroll_auto(
        container()
            .layout(LayoutStyle::flex_column().fill_width())
            .children_reactive(move || {
                let at = cursor.get();
                todos
                    .visible
                    .get()
                    .into_iter()
                    .enumerate()
                    .map(move |(index, todo)| {
                        let mark = if todo.done { "[x]" } else { "[ ]" };
                        let row = container()
                            .layout(LayoutStyle::flex_row().fill_width().padding_x(cols(1.0)));
                        let row = if index == at {
                            row.class("bg-accent-5")
                        } else {
                            row
                        };
                        row.child(text(format!("{mark} {}", todo.text)).nowrap().ellipsize())
                    })
                    .collect::<Vec<_>>()
            }),
    )
    .fill();
    container()
        .role(Role::List)
        .label("Todo items")
        .focusable(true)
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .min_height(rows(0.0)),
        )
        .on_added(move |_, id| list_id.set(Some(id)))
        .key(Key::Enter, move |_| todos.toggle())
        .key(LogicalKey::character(" "), move |_| todos.toggle())
        .key(Key::Delete, move |_| todos.remove())
        .key(LogicalKey::character("d"), move |_| todos.remove())
        .key(Key::ArrowDown, move |_| todos.step(1))
        .key(Key::ArrowUp, move |_| todos.step(-1))
        .key(LogicalKey::character("j"), move |_| todos.step(1))
        .key(LogicalKey::character("k"), move |_| todos.step(-1))
        .key(LogicalKey::character("f"), move |_| todos.cycle_filter())
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

pub(crate) fn app(cx: &mut Cx) -> NodeId {
    let todos = Todos::new();
    let draft = signal(String::new());
    let list_id = signal(None::<NodeId>);
    let quit = Shortcut::plain(LogicalKey::character("q"));
    cx.add(
        container()
            .layout(LayoutStyle::flex_column().fill())
            .keyboard_shortcut(quit.clone())
            .key(quit, |_| sigil::request_exit())
            .key_capture(Key::Escape, move |event| {
                if let Some(id) = list_id.peek() {
                    request_focus(id);
                    event.stop_propagation();
                }
            })
            .child(
                container()
                    .layout(LayoutStyle::flex_row().fill_width().gap(cols(2.0)))
                    .child(text("Todo").nowrap().class("text-accent"))
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
            .child(status(todos)),
    )
}
