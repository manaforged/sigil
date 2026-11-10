use core::time::Duration;

use crate::AppConfig;
use crate::animation::runtime::MotionPrefs;
use crate::events::ClickConfig;

#[derive(Default)]
struct PlatformPrefs {
    reduced_motion: Option<bool>,
    click_interval: Option<Duration>,
}

#[cfg(feature = "scene")]
fn apply_text(config: &AppConfig) {
    if let Some(fonts) = &config.text.fonts {
        crate::text::fonts::set_default_fonts(fonts.clone());
    }
    crate::text::fonts::set_fallback_fonts(&config.text.font_fallbacks);
    if let Some(tuning) = config.text.text_tuning {
        crate::renderer::slug::set_text_tuning(tuning);
    }
}

pub(crate) fn apply(config: &AppConfig) {
    let platform = read();
    let reduced = config
        .reduced_motion
        .or(platform.reduced_motion)
        .unwrap_or(false);
    MotionPrefs::set(reduced);
    let default = ClickConfig::default();
    let click = ClickConfig {
        interval: config
            .double_click_time
            .or(platform.click_interval)
            .unwrap_or(default.interval),
        distance: config.double_click_distance.unwrap_or(default.distance),
    };
    ClickConfig::set(click);
    #[cfg(feature = "scene")]
    apply_text(config);
}

#[cfg(target_os = "macos")]
fn read() -> PlatformPrefs {
    use objc2_app_kit::{NSEvent, NSWorkspace};
    let interval = NSEvent::doubleClickInterval();
    PlatformPrefs {
        reduced_motion: Some(
            NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion(),
        ),
        click_interval: Duration::try_from_secs_f64(interval).ok(),
    }
}

#[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
fn read() -> PlatformPrefs {
    PlatformPrefs {
        reduced_motion: gsetting("org.gnome.desktop.interface", "enable-animations")
            .and_then(|value| value.parse::<bool>().ok())
            .map(|enabled| !enabled),
        click_interval: gsetting("org.gnome.desktop.peripherals.mouse", "double-click")
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_millis),
    }
}

#[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
fn gsetting(schema: &str, key: &str) -> Option<String> {
    let output = std::process::Command::new("gsettings")
        .args(["get", schema, key])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    text.split_whitespace().last().map(str::to_owned)
}

#[cfg(not(unix))]
fn read() -> PlatformPrefs {
    PlatformPrefs::default()
}
