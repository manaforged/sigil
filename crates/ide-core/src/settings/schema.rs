//! Compile-time schema registry: known keys with typed defaults.
//!
//! The schema is the source of truth for which keys are valid. Anything
//! a TOML file sets that isn't in the schema becomes an
//! [`Diagnostic::UnknownKey`]; anything in the schema that no layer sets
//! resolves to its default.
//!
//! [`Diagnostic::UnknownKey`]: crate::settings::Diagnostic::UnknownKey

use std::collections::HashMap;

use crate::settings::value::{ErasedValue, SettingValue};

/// Parser shim: `(toml_value -> Option<ErasedValue>)`. Returning `None`
/// signals a TOML type mismatch versus the schema-registered type.
pub(super) type ParseFn = fn(&toml::Value) -> Option<ErasedValue>;

/// One schema entry: type label, default, and a parser bound to the
/// originally-registered type.
pub(super) struct Entry {
    pub(super) type_name: &'static str,
    pub(super) default: ErasedValue,
    pub(super) parse: ParseFn,
}

/// Schema registry: dotted-key → entry.
///
/// Built up via chained [`Schema::register`] calls, then handed to
/// [`crate::settings::Store::open`]. The registry is consumed (moved)
/// into the store, which owns it for the store's lifetime.
#[derive(Default)]
pub struct Schema {
    pub(super) entries: HashMap<String, Entry>,
}

impl Schema {
    /// Empty schema. Add keys with [`Self::register`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Register a setting key with its typed default.
    ///
    /// Re-registering the same key replaces the prior default. This is
    /// intentional — it lets layered constructors (a base set + an
    /// IDE-specific set) compose without merge gymnastics.
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
