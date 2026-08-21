//! To-do app state.
//!
//! All state lives on a `Copy` carrier of signals, so event handlers and
//! menu callbacks are plain `move ||` closures. The list persists to a
//! human-readable `[x] task` / `[ ] task` text file.

use std::fs;
use std::path::Path;

use sigil::prelude::*;

/// One to-do item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Todo {
    pub(crate) text: String,
    pub(crate) done: bool,
}

/// Which items the list shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Filter {
    All,
    Active,
    Done,
}

/// The whole app state. `Copy` — clone it into any closure freely.
#[derive(Clone, Copy)]
pub(crate) struct TodoState {
    pub(crate) todos: Signal<Vec<Todo>>,
    pub(crate) draft: Signal<String>,
    pub(crate) filter: Signal<Filter>,
    pub(crate) status: Signal<String>,
}

impl TodoState {
    /// Seed a few sample tasks so the window is not empty on launch.
    pub(crate) fn seed() -> Self {
        Self {
            todos: signal(vec![
                Todo {
                    text: String::from("Try the File menu \u{2192} Save\u{2026}"),
                    done: false,
                },
                Todo {
                    text: String::from("Toggle a task done"),
                    done: false,
                },
                Todo {
                    text: String::from("Ship color emoji \u{2705}"),
                    done: true,
                },
            ]),
            draft: signal(String::new()),
            filter: signal(Filter::All),
            status: signal(String::from("Ready. Type a task and press Enter.")),
        }
    }

    /// Append the current draft as a new task, if non-empty.
    pub(crate) fn add(self) {
        let text = self.draft.peek().trim().to_owned();
        if text.is_empty() {
            return;
        }
        self.todos.update(|todos| {
            todos.push(Todo {
                text: text.clone(),
                done: false,
            });
        });
        self.draft.set(String::new());
        self.status.set(format!("Added: {text}"));
    }

    /// Flip the done flag of the task at `index`.
    pub(crate) fn toggle(self, index: usize) {
        self.todos.update(|todos| {
            if let Some(todo) = todos.get_mut(index) {
                todo.done = !todo.done;
            }
        });
    }

    /// Remove the task at `index`.
    pub(crate) fn remove(self, index: usize) {
        self.todos.update(|todos| {
            if index < todos.len() {
                todos.remove(index);
            }
        });
        self.status.set(String::from("Removed a task."));
    }

    /// Drop every completed task.
    pub(crate) fn clear_completed(self) {
        self.todos.update(|todos| todos.retain(|todo| !todo.done));
        self.status.set(String::from("Cleared completed tasks."));
    }

    /// Empty the list (File \u{2192} New).
    pub(crate) fn new_list(self) {
        self.todos.set(Vec::new());
        self.draft.set(String::new());
        self.status.set(String::from("New list."));
    }

    /// Show which subset of the list.
    pub(crate) fn set_filter(self, filter: Filter) {
        self.filter.set(filter);
    }

    /// Count of not-yet-done tasks (subscribes, for a reactive label).
    pub(crate) fn active_count(self) -> usize {
        self.todos.get().iter().filter(|todo| !todo.done).count()
    }

    /// The `(original index, task)` pairs the current filter shows.
    /// Subscribes to both `todos` and `filter`, so a `children_reactive`
    /// list built from it rebuilds on any change.
    pub(crate) fn visible(self) -> Vec<(usize, Todo)> {
        let filter = self.filter.get();
        self.todos
            .get()
            .iter()
            .enumerate()
            .filter(|(_, todo)| match filter {
                Filter::All => true,
                Filter::Active => !todo.done,
                Filter::Done => todo.done,
            })
            .map(|(index, todo)| (index, todo.clone()))
            .collect()
    }

    /// Write the list to `path` as `[x] task` / `[ ] task` lines.
    pub(crate) fn save_to(self, path: &Path) {
        use std::fmt::Write as _;
        let mut body = String::new();
        for todo in &self.todos.peek() {
            let mark = if todo.done { 'x' } else { ' ' };
            let _ = writeln!(body, "[{mark}] {}", todo.text);
        }
        match fs::write(path, body) {
            Ok(()) => self.status.set(format!("Saved to {}", path.display())),
            Err(err) => self.status.set(format!("Save failed: {err}")),
        }
    }

    /// Replace the list with the contents of `path`.
    pub(crate) fn load_from(self, path: &Path) {
        match fs::read_to_string(path) {
            Ok(body) => {
                let todos = parse(&body);
                let count = todos.len();
                self.todos.set(todos);
                self.status
                    .set(format!("Loaded {count} tasks from {}", path.display()));
            }
            Err(err) => self.status.set(format!("Open failed: {err}")),
        }
    }
}

/// Parse the `[x] task` / `[ ] task` save format. Lines without the marker
/// are ignored, so hand-edited files degrade gracefully.
fn parse(body: &str) -> Vec<Todo> {
    body.lines()
        .filter_map(|line| {
            let done = line.starts_with("[x]") || line.starts_with("[X]");
            let marked =
                line.starts_with("[x]") || line.starts_with("[X]") || line.starts_with("[ ]");
            let text = line.get(3..).map(str::trim).unwrap_or_default();
            (marked && !text.is_empty()).then(|| Todo {
                text: text.to_owned(),
                done,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
