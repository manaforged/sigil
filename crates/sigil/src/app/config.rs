#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PanicPolicy {
    #[default]
    Isolate,
    Abort,
}

#[cfg(feature = "desktop")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Resize {
    Free,
    Fixed,
}

#[cfg(feature = "terminal")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OtherKeys {
    Plain,
    Modified,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct QuitKeys {
    pub(crate) esc: bool,
    pub(crate) ctrl_c: bool,
}

pub(crate) const FRAME_INTERVAL: std::time::Duration = std::time::Duration::from_millis(33);

#[cfg(feature = "terminal")]
#[derive(Clone, Default)]
pub(crate) struct TerminalHooks(
    std::rc::Rc<std::cell::RefCell<Option<crate::terminal::Hooks<'static>>>>,
);

#[cfg(feature = "terminal")]
impl TerminalHooks {
    pub(crate) fn take(&self) -> crate::terminal::Hooks<'static> {
        self.0.borrow_mut().take().unwrap_or_default()
    }
}

#[cfg(feature = "terminal")]
impl std::fmt::Debug for TerminalHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TerminalHooks")
    }
}

#[cfg(feature = "desktop")]
#[derive(Clone, Default)]
pub(crate) struct CloseHook(Option<std::rc::Rc<dyn Fn() -> crate::app::CloseAction>>);

#[cfg(feature = "desktop")]
impl CloseHook {
    pub(crate) fn decide(&self) -> crate::app::CloseAction {
        self.0
            .as_ref()
            .map_or(crate::app::CloseAction::Close, |hook| hook())
    }
}

#[cfg(feature = "desktop")]
impl std::fmt::Debug for CloseHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CloseHook")
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub(crate) title: String,
    pub(crate) size: (u32, u32),
    pub(crate) quit: QuitKeys,
    pub(crate) panic_policy: PanicPolicy,
    #[cfg(feature = "desktop")]
    pub(crate) overlay: bool,
    #[cfg(feature = "desktop")]
    pub(crate) frameless: bool,
    #[cfg(feature = "desktop")]
    pub(crate) anchor: Option<crate::app::Anchor>,
    pub(crate) theme_file: Option<std::path::PathBuf>,
    #[cfg(feature = "desktop")]
    pub(crate) user_zoom: f32,
    #[cfg(any(feature = "desktop", feature = "terminal"))]
    pub(crate) host: crate::kernel::HostMode,
    #[cfg(feature = "desktop")]
    pub(crate) min_size: Option<(u32, u32)>,
    #[cfg(feature = "desktop")]
    pub(crate) resize: Resize,
    #[cfg(feature = "desktop")]
    pub(crate) on_close: CloseHook,
    pub(crate) inline: bool,
    pub(crate) frame_interval: std::time::Duration,
    pub(crate) reduced_motion: Option<bool>,
    pub(crate) click: Option<crate::events::ClickConfig>,
    pub(crate) task_workers: crate::tasks::TaskWorkers,
    #[cfg(feature = "terminal")]
    pub(crate) terminal_hooks: TerminalHooks,
    #[cfg(feature = "terminal")]
    pub(crate) terminal_access: crate::terminal::Access,
    #[cfg(feature = "terminal")]
    pub(crate) mouse_tracking: Option<crate::terminal::MouseTracking>,
    #[cfg(feature = "terminal")]
    pub(crate) other_keys: OtherKeys,
    #[cfg(feature = "terminal")]
    pub(crate) suspend: Option<crate::events::Shortcut>,
    #[cfg(feature = "desktop")]
    #[cfg_attr(
        not(target_arch = "wasm32"),
        allow(dead_code, reason = "only the browser host binds a canvas")
    )]
    pub(crate) canvas: Option<String>,
}

impl AppConfig {
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            size: (720, 480),
            quit: QuitKeys {
                esc: false,
                ctrl_c: true,
            },
            panic_policy: PanicPolicy::default(),
            #[cfg(feature = "desktop")]
            overlay: false,
            #[cfg(feature = "desktop")]
            frameless: false,
            #[cfg(feature = "desktop")]
            anchor: None,
            theme_file: None,
            #[cfg(feature = "desktop")]
            user_zoom: 1.0,
            #[cfg(feature = "desktop")]
            host: crate::kernel::HostMode::Pixel,
            #[cfg(all(feature = "terminal", not(feature = "desktop")))]
            host: crate::kernel::HostMode::Cell,
            #[cfg(feature = "desktop")]
            min_size: None,
            #[cfg(feature = "desktop")]
            resize: Resize::Free,
            #[cfg(feature = "desktop")]
            on_close: CloseHook::default(),
            inline: false,
            frame_interval: FRAME_INTERVAL,
            reduced_motion: None,
            click: None,
            task_workers: crate::tasks::TaskWorkers::Auto,
            #[cfg(feature = "terminal")]
            terminal_hooks: TerminalHooks::default(),
            #[cfg(feature = "terminal")]
            terminal_access: crate::terminal::Access::default(),
            #[cfg(feature = "terminal")]
            mouse_tracking: None,
            #[cfg(feature = "terminal")]
            other_keys: OtherKeys::Plain,
            #[cfg(feature = "terminal")]
            suspend: Some(crate::events::Shortcut::ctrl(
                crate::events::LogicalKey::character("z"),
            )),
            #[cfg(feature = "desktop")]
            canvas: None,
        }
    }

    #[must_use]
    pub const fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    #[must_use]
    pub const fn esc_exits(mut self, yes: bool) -> Self {
        self.quit.esc = yes;
        self
    }

    #[must_use]
    pub const fn quit_on_ctrl_c(mut self, yes: bool) -> Self {
        self.quit.ctrl_c = yes;
        self
    }

    #[must_use]
    pub const fn panic_policy(mut self, policy: PanicPolicy) -> Self {
        self.panic_policy = policy;
        self
    }

    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn overlay(mut self, yes: bool) -> Self {
        self.overlay = yes;
        self
    }

    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn frameless(mut self, yes: bool) -> Self {
        self.frameless = yes;
        self
    }

    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn anchor(mut self, anchor: crate::app::Anchor) -> Self {
        self.anchor = Some(anchor);
        self
    }

    #[must_use]
    pub fn theme_file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.theme_file = Some(path.into());
        self
    }

    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn window(mut self) -> Self {
        self.host = crate::kernel::HostMode::Pixel;
        self
    }

    #[cfg(feature = "desktop")]
    #[must_use]
    pub fn canvas(mut self, id: impl Into<String>) -> Self {
        self.canvas = Some(id.into());
        self
    }

    #[cfg(feature = "terminal")]
    #[must_use]
    pub const fn terminal(mut self) -> Self {
        self.host = crate::kernel::HostMode::Cell;
        self
    }

    #[cfg(feature = "terminal")]
    #[must_use]
    pub fn terminal_hooks(mut self, hooks: crate::terminal::Hooks<'static>) -> Self {
        self.terminal_hooks = TerminalHooks(std::rc::Rc::new(std::cell::RefCell::new(Some(hooks))));
        self
    }

    #[cfg(feature = "terminal")]
    #[must_use]
    pub const fn terminal_access(mut self, access: crate::terminal::Access) -> Self {
        self.terminal_access = access;
        self
    }

    #[cfg(feature = "terminal")]
    #[must_use]
    pub const fn mouse_tracking(mut self, tracking: crate::terminal::MouseTracking) -> Self {
        self.mouse_tracking = Some(tracking);
        self
    }

    #[cfg(feature = "terminal")]
    #[must_use]
    pub const fn modify_other_keys(mut self, yes: bool) -> Self {
        self.other_keys = if yes {
            OtherKeys::Modified
        } else {
            OtherKeys::Plain
        };
        self
    }

    #[cfg(feature = "terminal")]
    #[must_use]
    pub fn suspend_on(mut self, shortcut: impl Into<Option<crate::events::Shortcut>>) -> Self {
        self.suspend = shortcut.into();
        self
    }

    #[must_use]
    pub const fn inline(mut self, yes: bool) -> Self {
        self.inline = yes;
        self
    }

    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn user_zoom(mut self, zoom: f32) -> Self {
        self.user_zoom = zoom;
        self
    }

    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn min_size(mut self, width: u32, height: u32) -> Self {
        self.min_size = Some((width, height));
        self
    }

    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn resizable(mut self, yes: bool) -> Self {
        self.resize = if yes { Resize::Free } else { Resize::Fixed };
        self
    }

    #[cfg(feature = "desktop")]
    #[must_use]
    pub fn on_close(mut self, decide: impl Fn() -> crate::app::CloseAction + 'static) -> Self {
        self.on_close = CloseHook(Some(std::rc::Rc::new(decide)));
        self
    }

    #[must_use]
    pub const fn frame_interval(mut self, interval: std::time::Duration) -> Self {
        self.frame_interval = interval;
        self
    }

    #[must_use]
    pub const fn reduced_motion(mut self, reduced: bool) -> Self {
        self.reduced_motion = Some(reduced);
        self
    }

    #[must_use]
    pub const fn click(mut self, click: crate::events::ClickConfig) -> Self {
        self.click = Some(click);
        self
    }

    #[must_use]
    pub const fn task_workers(mut self, workers: crate::tasks::TaskWorkers) -> Self {
        self.task_workers = workers;
        self
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub const fn window_size(&self) -> (u32, u32) {
        self.size
    }

    #[must_use]
    pub const fn exits_on_esc(&self) -> bool {
        self.quit.esc
    }

    #[must_use]
    pub const fn on_panic(&self) -> PanicPolicy {
        self.panic_policy
    }
}

impl From<&str> for AppConfig {
    fn from(title: &str) -> Self {
        Self::new(title)
    }
}

impl From<String> for AppConfig {
    fn from(title: String) -> Self {
        Self::new(title)
    }
}
