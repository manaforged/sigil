#[cfg(feature = "scene")]
use super::text::TextSetup;
use super::*;

impl AppConfig {
    pub(super) fn defaults() -> Self {
        Self {
            #[cfg(app_host)]
            title: String::new(),
            size: (720, 480),
            quit: QUIT_KEYS,
            panic_policy: PanicPolicy::default(),
            #[cfg(feature = "desktop")]
            overlay: false,
            #[cfg(feature = "desktop")]
            frameless: false,
            #[cfg(feature = "desktop")]
            inset_title_bar: None,
            #[cfg(feature = "desktop")]
            anchor: None,
            theme_file: None,
            #[cfg(feature = "desktop")]
            user_zoom: 1.0,
            #[cfg(app_host)]
            host: HOST_MODE,
            #[cfg(feature = "desktop")]
            min_size: None,
            #[cfg(feature = "desktop")]
            resize: Resize::Free,
            #[cfg(feature = "desktop")]
            on_close: CloseHook::default(),
            #[cfg(app_host)]
            on_exit: ExitHook::default(),
            inline: false,
            frame_interval: FRAME_INTERVAL,
            reduced_motion: None,
            double_click_time: None,
            double_click_distance: None,
            task_workers: crate::tasks::TaskWorkers::Auto,
            #[cfg(feature = "scene")]
            text: TextSetup::default(),
            #[cfg(feature = "terminal")]
            terminal: TerminalOptions::default(),
            #[cfg(feature = "desktop")]
            canvas: None,
        }
    }
}
