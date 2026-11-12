use super::{AppConfig, PanicPolicy};

#[test]
fn a_title_keeps_the_defaults() {
    let cfg: AppConfig = "Demo".into();
    assert_eq!(cfg.title, "Demo");
    assert_eq!(cfg.size, (720, 480));
    assert!(!cfg.quit.esc);
    assert_eq!(cfg.panic_policy, PanicPolicy::Isolate);
}

#[test]
fn from_string_works_too() {
    let cfg: AppConfig = String::from("Owned").into();
    assert_eq!(cfg.title, "Owned");
}

#[test]
fn builder_chain_overrides_individual_fields() {
    let cfg = AppConfig::new("Chain")
        .size(640, 360)
        .exit_on_esc(false)
        .panic_policy(PanicPolicy::Abort);
    assert_eq!(cfg.title, "Chain");
    assert_eq!(cfg.size, (640, 360));
    assert!(!cfg.quit.esc);
    assert_eq!(cfg.panic_policy, PanicPolicy::Abort);
}

#[cfg(feature = "scene")]
#[test]
fn a_fallback_face_keeps_the_primary_font_source() {
    let cfg = AppConfig::new("Fonts").font_fallback(vec![1, 2, 3], 7);
    assert!(cfg.text.fonts.is_none());
    assert_eq!(cfg.text.font_fallbacks.len(), 1);
    assert_eq!(cfg.text.font_fallbacks.first().map(|face| face.1), Some(7));
}

const GAP: std::time::Duration = std::time::Duration::from_millis(12);
const PAST: std::time::Duration = std::time::Duration::from_millis(13);

#[test]
fn double_click_limits_follow_the_setter() {
    let config = AppConfig::new("t")
        .double_click_time(GAP)
        .double_click_distance(3.0);
    assert_eq!(config.double_click_time, Some(GAP));
    assert_eq!(config.double_click_distance, Some(3.0));
    let limits = crate::events::ClickConfig {
        interval: config.double_click_time.expect("time"),
        distance: config.double_click_distance.expect("distance"),
    };
    assert!(limits.is_repeat(
        GAP,
        crate::geometry::Point { x: 0.0, y: 0.0 },
        crate::geometry::Point { x: 3.0, y: 0.0 },
    ));
    assert!(!limits.is_repeat(
        PAST,
        crate::geometry::Point { x: 0.0, y: 0.0 },
        crate::geometry::Point { x: 0.0, y: 0.0 },
    ));
}
