use std::any::Any;
use std::sync::Arc;

pub(super) type ErasedValue = Arc<dyn Any + Send + Sync>;

pub trait SettingValue: Clone + Send + Sync + 'static {
    fn type_name() -> &'static str;

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
