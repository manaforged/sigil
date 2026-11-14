use core::cell::Cell;
use core::time::Duration;

use crate::AppConfig;
use crate::animation::runtime::MotionPrefs;
use crate::events::ClickConfig;

#[derive(Clone, Copy)]
struct Access {
    reduced_motion: Option<bool>,
    reduced_transparency: Option<bool>,
}

const UNSET: Access = Access {
    reduced_motion: None,
    reduced_transparency: None,
};

impl Access {
    const fn pinned(self) -> bool {
        self.reduced_motion.is_some() && self.reduced_transparency.is_some()
    }

    fn or(self, fallback: Self) -> Self {
        Self {
            reduced_motion: self.reduced_motion.or(fallback.reduced_motion),
            reduced_transparency: self.reduced_transparency.or(fallback.reduced_transparency),
        }
    }

    fn apply(self) {
        MotionPrefs::set(self.reduced_motion.unwrap_or(false));
        MotionPrefs::transparency(self.reduced_transparency.unwrap_or(false));
    }
}

thread_local! {
    static PINNED: Cell<Access> = const { Cell::new(UNSET) };
}

#[cfg(feature = "scene")]
fn apply_text(config: &AppConfig) {
    if let Some(fonts) = &config.text.fonts {
        crate::text::fonts::default_fonts(fonts.clone());
    }
    crate::text::fonts::set_fallback_fonts(&config.text.font_fallbacks);
    if let Some(fonts) = &config.text.web_fonts {
        crate::text::set_web_fallbacks(fonts.iter().cloned());
    }
}

pub(crate) fn apply(config: &AppConfig) {
    PINNED.with(|cell| {
        cell.set(Access {
            reduced_motion: config.reduced_motion,
            reduced_transparency: config.reduced_transparency,
        });
    });
    refresh();
    let default = ClickConfig::default();
    let click = ClickConfig {
        interval: config
            .double_click_time
            .or_else(read_click_interval)
            .unwrap_or(default.interval),
        distance: config.double_click_distance.unwrap_or(default.distance),
    };
    ClickConfig::set(click);
    #[cfg(feature = "scene")]
    apply_text(config);
}

pub(crate) fn refresh() {
    let pinned = PINNED.with(Cell::get);
    let platform = if pinned.pinned() {
        UNSET
    } else {
        read_access()
    };
    pinned.or(platform).apply();
}

#[cfg(target_os = "macos")]
fn read_access() -> Access {
    let workspace = objc2_app_kit::NSWorkspace::sharedWorkspace();
    Access {
        reduced_motion: Some(workspace.accessibilityDisplayShouldReduceMotion()),
        reduced_transparency: Some(workspace.accessibilityDisplayShouldReduceTransparency()),
    }
}

#[cfg(target_os = "macos")]
fn read_click_interval() -> Option<Duration> {
    Duration::try_from_secs_f64(objc2_app_kit::NSEvent::doubleClickInterval()).ok()
}

#[cfg(freedesktop)]
fn read_access() -> Access {
    Access {
        reduced_motion: gsetting("org.gnome.desktop.interface", "enable-animations")
            .and_then(|value| value.parse::<bool>().ok())
            .map(|enabled| !enabled),
        reduced_transparency: None,
    }
}

#[cfg(freedesktop)]
fn read_click_interval() -> Option<Duration> {
    gsetting("org.gnome.desktop.peripherals.mouse", "double-click")
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
}

#[cfg(freedesktop)]
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
const fn read_access() -> Access {
    UNSET
}

#[cfg(not(unix))]
const fn read_click_interval() -> Option<Duration> {
    None
}
