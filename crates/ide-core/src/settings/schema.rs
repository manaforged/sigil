use std::collections::HashMap;

use crate::settings::value::{ErasedValue, SettingValue};

pub(super) type ParseFn = fn(&toml::Value) -> Option<ErasedValue>;

pub(super) struct Entry {
    pub(super) type_name: &'static str,
    pub(super) default: ErasedValue,
    pub(super) parse: ParseFn,
}

#[derive(Default)]
pub struct Schema {
    pub(super) entries: HashMap<String, Entry>,
}

impl Schema {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn register<T: SettingValue>(&mut self, key: &str, default: T) -> &mut Self {
        let entry = Entry {
            type_name: T::type_name(),
            default: std::sync::Arc::new(default),
            parse: parse_to_erased::<T>,
        };
        self.entries.insert(key.to_string(), entry);
        self
    }
}

fn parse_to_erased<T: SettingValue>(v: &toml::Value) -> Option<ErasedValue> {
    T::from_toml(v).map(|val| std::sync::Arc::new(val) as ErasedValue)
}
