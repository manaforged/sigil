#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::AppConfig;
use sigil::app::PanicPolicy;

#[test]
fn a_title_keeps_the_defaults() {
    let cfg: AppConfig = "Demo".into();
    assert_eq!(cfg.title(), "Demo");
    assert_eq!(cfg.window_size(), (720, 480));
    assert!(!cfg.exits_on_esc());
    assert_eq!(cfg.on_panic(), PanicPolicy::Isolate);
}

#[test]
fn from_string_works_too() {
    let cfg: AppConfig = String::from("Owned").into();
    assert_eq!(cfg.title(), "Owned");
}

#[test]
fn builder_chain_overrides_individual_fields() {
    let cfg = AppConfig::new("Chain")
        .size(640, 360)
        .esc_exits(false)
        .panic_policy(PanicPolicy::Abort);
    assert_eq!(cfg.title(), "Chain");
    assert_eq!(cfg.window_size(), (640, 360));
    assert!(!cfg.exits_on_esc());
    assert_eq!(cfg.on_panic(), PanicPolicy::Abort);
}

#[test]
fn default_panic_policy_is_isolate() {
    assert_eq!(PanicPolicy::default(), PanicPolicy::Isolate);
}
