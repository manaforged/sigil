#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use sigil_app::{AppConfig, PanicPolicy};

#[test]
fn from_unit_uses_defaults() {
    let cfg: AppConfig = ().into();
    assert_eq!(cfg.title, "Sigil");
    assert_eq!(cfg.size, (720, 480));
    assert!(cfg.esc_exits);
    assert_eq!(cfg.panic_policy, PanicPolicy::Isolate);
}

#[test]
fn from_str_keeps_defaults_for_size() {
    let cfg: AppConfig = "Demo".into();
    assert_eq!(cfg.title, "Demo");
    assert_eq!(cfg.size, (720, 480));
}

#[test]
fn from_string_works_too() {
    let cfg: AppConfig = String::from("Owned").into();
    assert_eq!(cfg.title, "Owned");
}

#[test]
fn from_str_size_tuple_overrides_size() {
    let cfg: AppConfig = ("Big", 1920, 1080).into();
    assert_eq!(cfg.title, "Big");
    assert_eq!(cfg.size, (1920, 1080));
}

#[test]
fn builder_chain_overrides_individual_fields() {
    let cfg = AppConfig::new("Chain")
        .size(640, 360)
        .esc_exits(false)
        .panic_policy(PanicPolicy::Abort);
    assert_eq!(cfg.title, "Chain");
    assert_eq!(cfg.size, (640, 360));
    assert!(!cfg.esc_exits);
    assert_eq!(cfg.panic_policy, PanicPolicy::Abort);
}

#[test]
fn default_panic_policy_is_isolate() {
    assert_eq!(PanicPolicy::default(), PanicPolicy::Isolate);
}

#[test]
fn default_app_config_matches_unit_into() {
    let from_default = AppConfig::default();
    let from_unit: AppConfig = ().into();
    assert_eq!(from_default.title, from_unit.title);
    assert_eq!(from_default.size, from_unit.size);
    assert_eq!(from_default.esc_exits, from_unit.esc_exits);
    assert_eq!(from_default.panic_policy, from_unit.panic_policy);
}
