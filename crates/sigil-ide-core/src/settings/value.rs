//! Typed value trait for settings.
//!
//! Limited to TOML-representable primitives. Implemented for `bool`,
//! `i64`, `f64`, `String`, and `Vec<String>`. Extending the set later
//! requires only adding a new `impl SettingValue` — no changes to the
//! [`crate::settings::Store`] type machinery.

use std::any::Any;
use std::sync::Arc;

/// Type-erased value held in the resolved-cache. `Arc<dyn Any>` keeps
/// the store generic-free internally while preserving cheap cloning
/// across watch dispatch.
pub(super) type ErasedValue = Arc<dyn Any + Send + Sync>;

/// Marker trait for valid setting value types.
///
/// Implementors must be cloneable, send-and-sync (so watch callbacks
/// can fire across threads), and parsable from a `toml::Value`.
/// Returning `None` from [`SettingValue::from_toml`] signals a type
/// mismatch — the store turns that into a [`Diagnostic::TypeMismatch`].
///
/// [`Diagnostic::TypeMismatch`]: crate::settings::Diagnostic::TypeMismatch
pub trait SettingValue: Clone + Send + Sync + 'static {
    /// TOML type label used in diagnostics (e.g. `"integer"`).
    fn type_name() -> &'static str;

    /// Parse this value from a `toml::Value`. `None` on type mismatch.
    fn from_toml(value: &toml::Value) -> Option<Self>;
}

impl SettingValue for bool {
    fn type_name() -> &'static str {
        "bool"
    }
    fn from_toml(value: &toml::Value) -> Option<Self> {
        value.as_bool()
    }
}

impl SettingValue for i64 {
    fn type_name() -> &'static str {
        "integer"
    }
    fn from_toml(value: &toml::Value) -> Option<Self> {
        value.as_integer()
    }
}

impl SettingValue for f64 {
    fn type_name() -> &'static str {
        "float"
    }
    fn from_toml(value: &toml::Value) -> Option<Self> {
        value.as_float()
    }
}

impl SettingValue for String {
    fn type_name() -> &'static str {
        "string"
    }
    fn from_toml(value: &toml::Value) -> Option<Self> {
        value.as_str().map(std::string::ToString::to_string)
    }
}

impl SettingValue for Vec<String> {
    fn type_name() -> &'static str {
        "array<string>"
    }
    fn from_toml(value: &toml::Value) -> Option<Self> {
        let arr = value.as_array()?;
        arr.iter().map(|v| v.as_str().map(String::from)).collect()
    }
}
