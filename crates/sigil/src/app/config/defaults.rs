use super::*;

impl AppConfig {
    pub(super) fn defaults() -> Self {
        Self {
            #[cfg(any(test, feature = "desktop", feature = "terminal"))]
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
            #[cfg(any(feature = "desktop", feature = "terminal"))]
            host: HOST_MODE,
            #[cfg(feature = "desktop")]
            min_size: None,
            #[cfg(feature = "desktop")]
            resize: Resize::Free,
            #[cfg(feature = "desktop")]
            on_close: CloseHook::default(),
            #[cfg(feature = "desktop")]
            on_exit: ExitHook::default(),
            inline: false,
            frame_interval: FRAME_INTERVAL,
            reduced_motion: None,
            double_click_time: None,
            double_click_distance: None,
            task_workers: crate::tasks::TaskWorkers::Auto,
            #[cfg(feature = "scene")]
            fonts: None,
            #[cfg(feature = "scene")]
            font_fallbacks: Vec::new(),
            #[cfg(feature = "scene")]
            text_tuning: None,
            #[cfg(feature = "terminal")]
            terminal_hooks: TerminalHooks::default(),
            #[cfg(feature = "terminal")]
            terminal_access: crate::terminal::Access::default(),
            #[cfg(feature = "terminal")]
            mouse_tracking: None,
            #[cfg(feature = "terminal")]
            other_keys: OtherKeys::Plain,
            #[cfg(feature = "terminal")]
            suspend: Some(suspend_key()),
            #[cfg(feature = "desktop")]
            canvas: None,
        }
    }
}
