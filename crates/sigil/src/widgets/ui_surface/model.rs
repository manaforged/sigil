use serde_json::{Map, Value};

use crate::reactive::{Signal, signal, untrack};

#[derive(Clone, Copy)]
pub(super) struct Model {
    data: Signal<Value>,
}

impl Model {
    pub(super) fn new(data: Value) -> Self {
        Self { data: signal(data) }
    }

    pub(super) fn get(self, path: &str) -> Value {
        self.data
            .with(|root| lookup(root, path).cloned().unwrap_or(Value::Null))
    }

    pub(super) fn peek(self, path: &str) -> Value {
        untrack(|| self.get(path))
    }

    pub(super) fn set(self, path: &str, value: Value) {
        if self.peek(path) != value {
            self.data.update(|root| write(root, path, Some(value)));
        }
    }
}

pub(super) fn lookup<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() || path == "/" {
        return Some(root);
    }
    root.pointer(path)
}

fn token(raw: &str) -> String {
    raw.replace("~1", "/").replace("~0", "~")
}

fn step<'a>(node: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    if !node.is_object() && !node.is_array() {
        *node = Value::Object(Map::new());
    }
    match node {
        Value::Object(map) => Some(map.entry(key.to_string()).or_insert(Value::Null)),
        Value::Array(items) => {
            let index = key.parse::<usize>().ok()?;
            if index == items.len() {
                items.push(Value::Null);
            }
            items.get_mut(index)
        }
        _ => None,
    }
}

fn place(node: &mut Value, key: &str, value: Option<Value>) {
    match (node, value) {
        (Value::Object(map), Some(value)) => {
            map.insert(key.to_string(), value);
        }
        (Value::Object(map), None) => {
            map.remove(key);
        }
        (Value::Array(items), value) => {
            let Ok(index) = key.parse::<usize>() else {
                return;
            };
            match value {
                Some(value) if index < items.len() => items[index] = value,
                Some(value) if index == items.len() => items.push(value),
                None if index < items.len() => {
                    items.remove(index);
                }
                _ => {}
            }
        }
        (node, Some(value)) => {
            let mut map = Map::new();
            map.insert(key.to_string(), value);
            *node = Value::Object(map);
        }
        (_, None) => {}
    }
}

pub(super) fn write(root: &mut Value, path: &str, value: Option<Value>) {
    let keys: Vec<String> = path.split('/').skip(1).map(token).collect();
    let whole = path.is_empty() || path == "/";
    let Some((last, parents)) = keys.split_last().filter(|_| !whole) else {
        *root = value.unwrap_or(Value::Null);
        return;
    };
    let mut node = root;
    for key in parents {
        let Some(next) = step(node, key) else { return };
        node = next;
    }
    place(node, last, value);
}

pub(super) fn display(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Bool(_) | Value::Number(_) | Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}
