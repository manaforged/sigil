use serde_json::{Map, Value};

use crate::reactive::{Signal, signal, untrack};

use super::document::{MAX_DEPTH, Problem};

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

    pub(super) fn len(self, path: &str) -> usize {
        self.data.with(|root| match lookup(root, path) {
            Some(Value::Array(items)) => items.len(),
            _ => 0,
        })
    }

    pub(super) fn set(self, path: &str, value: Value) {
        if pointer(path).is_ok() && self.peek(path) != value {
            self.data.update(|root| {
                write(root, path, Some(value)).ok();
            });
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

fn pointer(path: &str) -> Result<Vec<String>, Problem> {
    if path.is_empty() || path == "/" {
        return Ok(Vec::new());
    }
    if !path.starts_with('/') {
        return Err(Problem::BadPath);
    }
    if path.bytes().filter(|byte| *byte == b'/').count() > MAX_DEPTH {
        return Err(Problem::TooDeep);
    }
    Ok(path.split('/').skip(1).map(token).collect())
}

fn index(key: &str) -> Option<usize> {
    let digits = !key.is_empty() && key.bytes().all(|byte| byte.is_ascii_digit());
    let canonical = key == "0" || !key.starts_with('0');
    if digits && canonical {
        key.parse().ok()
    } else {
        None
    }
}

fn step<'a>(node: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    if !node.is_object() && !node.is_array() {
        *node = Value::Object(Map::new());
    }
    match node {
        Value::Object(map) => Some(map.entry(key.to_string()).or_insert(Value::Null)),
        Value::Array(items) => {
            let index = index(key)?;
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
            let Some(index) = index(key) else {
                return;
            };
            let len = items.len();
            match (value, items.get_mut(index)) {
                (Some(value), Some(slot)) => *slot = value,
                (Some(value), None) if index == len => items.push(value),
                (None, Some(_)) => {
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

pub(super) fn write(root: &mut Value, path: &str, value: Option<Value>) -> Result<(), Problem> {
    let keys = pointer(path)?;
    let Some((last, parents)) = keys.split_last() else {
        *root = value.unwrap_or(Value::Null);
        return Ok(());
    };
    let mut node = root;
    for key in parents {
        let Some(next) = step(node, key) else {
            return Ok(());
        };
        node = next;
    }
    place(node, last, value);
    Ok(())
}

pub(super) fn display(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Bool(_) | Value::Number(_) | Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}
