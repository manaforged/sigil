use std::collections::HashMap;

use serde_json::{Map, Value};

use super::model::write;
use super::spec::Component;

pub(super) const MAX_BYTES: usize = 256 << 10;
pub(super) const MAX_DEPTH: usize = 32;
pub(super) const MAX_COMPONENTS: usize = 512;
const MAX_MESSAGES: usize = 256;

pub(super) struct Surface {
    pub(super) components: HashMap<String, Component>,
    pub(super) data: Value,
}

pub(super) enum Problem {
    TooLarge,
    TooDeep,
    TooMany,
    BadPath,
    Json(String),
    NoMessages,
    NoRoot,
}

impl Problem {
    pub(super) fn message(&self) -> String {
        match self {
            Self::TooLarge => format!("UI surface is larger than {} KiB.", MAX_BYTES >> 10),
            Self::TooDeep => format!("UI surface nests deeper than {MAX_DEPTH} levels."),
            Self::TooMany => format!("UI surface has more than {MAX_COMPONENTS} components."),
            Self::BadPath => "UI surface has a data path that is not a JSON Pointer.".to_string(),
            Self::Json(error) => format!("UI surface is not valid JSON: {error}"),
            Self::NoMessages => "UI surface has no A2UI messages.".to_string(),
            Self::NoRoot => "UI surface has no component with id \"root\".".to_string(),
        }
    }
}

fn depth(value: &Value) -> usize {
    let mut deepest = 0;
    let mut stack = vec![(value, 1)];
    while let Some((node, level)) = stack.pop() {
        deepest = deepest.max(level);
        match node {
            Value::Array(items) => stack.extend(items.iter().map(|item| (item, level + 1))),
            Value::Object(map) => stack.extend(map.values().map(|item| (item, level + 1))),
            _ => {}
        }
    }
    deepest
}

fn messages(root: &Value) -> Vec<&Map<String, Value>> {
    let list = match root {
        Value::Array(items) => items.as_slice(),
        Value::Object(map) => match map.get("messages") {
            Some(Value::Array(items)) => items.as_slice(),
            _ => std::slice::from_ref(root),
        },
        _ => &[],
    };
    list.iter()
        .take(MAX_MESSAGES)
        .filter_map(Value::as_object)
        .collect()
}

impl Surface {
    fn apply(&mut self, message: &Map<String, Value>) -> Result<(), Problem> {
        if let Some(update) = message.get("updateComponents") {
            let list = update.get("components").and_then(Value::as_array);
            for props in list.into_iter().flatten().filter_map(Value::as_object) {
                let Some(id) = props.get("id").and_then(Value::as_str) else {
                    continue;
                };
                self.components
                    .insert(id.to_string(), Component::parse(props));
                if self.components.len() > MAX_COMPONENTS {
                    return Err(Problem::TooMany);
                }
            }
        }
        if let Some(update) = message.get("updateDataModel") {
            let path = update.get("path").and_then(Value::as_str).unwrap_or("/");
            write(&mut self.data, path, update.get("value").cloned())?;
        }
        if message.contains_key("deleteSurface") {
            self.components.clear();
            self.data = Value::Object(Map::new());
        }
        Ok(())
    }
}

pub(super) fn parse(source: &str) -> Result<Surface, Problem> {
    if source.len() > MAX_BYTES {
        return Err(Problem::TooLarge);
    }
    let root: Value =
        serde_json::from_str(source).map_err(|error| Problem::Json(error.to_string()))?;
    if depth(&root) > MAX_DEPTH {
        return Err(Problem::TooDeep);
    }
    let list = messages(&root);
    if list.is_empty() {
        return Err(Problem::NoMessages);
    }
    let mut surface = Surface {
        components: HashMap::new(),
        data: Value::Object(Map::new()),
    };
    for message in list {
        surface.apply(message)?;
    }
    if !surface.components.contains_key("root") {
        return Err(Problem::NoRoot);
    }
    Ok(surface)
}
