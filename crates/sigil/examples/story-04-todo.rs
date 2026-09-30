use sigil::prelude::*;

fn main() -> AppResult {
    run("Todo", todo)
}

#[derive(Clone)]
struct Todo {
    id: u64,
    text: String,
    done: bool,
}

pub(crate) fn todo() -> impl IntoNode {
    let draft = signal(String::new());
    let todos = signal(Vec::<Todo>::new());
    let next = signal(0_u64);
    let add = move || {
        let text = draft.peek().trim().to_string();
        if text.is_empty() {
            return;
        }
        let id = next.peek();
        next.set(id + 1);
        todos.update(|list| {
            list.push(Todo {
                id,
                text,
                done: false,
            });
        });
        draft.set(String::new());
    };
    let left = move || todos.with(|list| list.iter().filter(|todo| !todo.done).count());
    column()
        .child(
            row()
                .child(
                    text_field(draft)
                        .placeholder("New item")
                        .submit(move |_| add()),
                )
                .child(button("Add").activate(add)),
        )
        .child(scroll(for_each(
            todos,
            |todo: &Todo| todo.id,
            move |todo: &Todo| item(todos, todo.clone()),
        )))
        .child(text(move || format!("{} left", left())))
}

fn item(todos: Signal<Vec<Todo>>, entry: Todo) -> impl IntoNode {
    let id = entry.id;
    let done = signal(entry.done);
    let toggle = move |value| {
        todos.update(|list| {
            if let Some(todo) = list.iter_mut().find(|todo| todo.id == id) {
                todo.done = value;
            }
        });
    };
    row()
        .child(checkbox(done).label(entry.text).change(toggle))
        .child(
            button("Delete")
                .activate(move || todos.update(|list| list.retain(|todo| todo.id != id))),
        )
}
