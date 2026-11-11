use crate::app::Wake;
use crate::app::bridge::Native;
use crate::app::menu::Menu;
use crate::error::{AppResult, Error, ErrorKind};
use crate::events::MouseButton;
use crate::geometry::Point;
use crate::reactive::EventChannel;
use tray_icon::{Icon, MouseButton as NativeButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

thread_local! {
    static NATIVE: Native<Tray, TrayIcon, TrayEvent> = Native::new();
}

#[derive(Clone, Debug, Default)]
enum Change<T> {
    #[default]
    Keep,
    Set(T),
    Clear,
}

/// A system tray icon.
#[derive(Clone, Debug, Default)]
pub struct Tray {
    icon: Change<(Vec<u8>, u32, u32)>,
    tooltip: Change<String>,
    title: Change<String>,
    menu: Change<Menu>,
}

impl Tray {
    /// A tray icon with nothing set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            icon: Change::Keep,
            tooltip: Change::Keep,
            title: Change::Keep,
            menu: Change::Keep,
        }
    }

    /// Sets the image from RGBA bytes.
    #[must_use]
    pub fn icon(mut self, rgba: Vec<u8>, width: u32, height: u32) -> Self {
        self.icon = Change::Set((rgba, width, height));
        self
    }

    /// Makes `update` remove the installed image.
    #[must_use]
    pub fn clear_icon(mut self) -> Self {
        self.icon = Change::Clear;
        self
    }

    /// Sets the text shown on hover.
    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Change::Set(tooltip.into());
        self
    }

    /// Makes `update` remove the installed tooltip.
    #[must_use]
    pub fn clear_tooltip(mut self) -> Self {
        self.tooltip = Change::Clear;
        self
    }

    /// Sets the text shown beside the icon.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Change::Set(title.into());
        self
    }

    /// Makes `update` remove the installed title.
    #[must_use]
    pub fn clear_title(mut self) -> Self {
        self.title = Change::Clear;
        self
    }

    /// Sets the menu the icon opens.
    #[must_use]
    pub fn menu(mut self, menu: Menu) -> Self {
        self.menu = Change::Set(menu);
        self
    }

    /// Makes `update` remove the installed menu.
    #[must_use]
    pub fn clear_menu(mut self) -> Self {
        self.menu = Change::Clear;
        self
    }

    /// Queues the tray icon for the next frame; an error means the icon image is invalid.
    pub fn install(self) -> AppResult<()> {
        self.native_icon()
            .map_err(|error| Error::message(ErrorKind::Other, error))?;
        NATIVE.with(|native| native.queue(self));
        Ok(())
    }

    /// Applies the set and cleared fields to the installed icon; `Ok(false)` before it is installed.
    pub fn update(&self) -> AppResult<bool> {
        NATIVE.with(|native| {
            native.installed(|slot| -> AppResult<bool> {
                let Some(icon) = slot.as_ref() else {
                    return Ok(false);
                };
                self.apply(icon)
                    .map_err(|error| Error::message(ErrorKind::Other, error))?;
                Ok(true)
            })
        })
    }

    /// The pointer events on the tray icon.
    #[must_use]
    pub fn events() -> EventChannel<TrayEvent> {
        NATIVE.with(Native::events)
    }

    fn apply(&self, native: &TrayIcon) -> Result<(), String> {
        let icon = self.native_icon()?;
        let menu = self.native_menu()?;
        if icon.is_some() || matches!(self.icon, Change::Clear) {
            native.set_icon(icon).map_err(|error| error.to_string())?;
        }
        match &self.tooltip {
            Change::Keep => {}
            Change::Set(tooltip) => native
                .set_tooltip(Some(tooltip))
                .map_err(|error| error.to_string())?,
            Change::Clear => native
                .set_tooltip(None::<&str>)
                .map_err(|error| error.to_string())?,
        }
        match &self.title {
            Change::Keep => {}
            Change::Set(title) => native.set_title(Some(title)),
            Change::Clear => native.set_title(None::<&str>),
        }
        if let Some(menu) = menu {
            native.set_menu(Some(Box::new(menu)));
        } else if matches!(self.menu, Change::Clear) {
            native.set_menu(None);
        }
        Ok(())
    }

    fn native(self) -> Result<TrayIcon, String> {
        let mut builder = TrayIconBuilder::new();
        if let Some(menu) = self.native_menu()? {
            builder = builder.with_menu(Box::new(menu));
        }
        if let Some(icon) = self.native_icon()? {
            builder = builder.with_icon(icon);
        }
        if let Change::Set(tooltip) = self.tooltip {
            builder = builder.with_tooltip(tooltip);
        }
        if let Change::Set(title) = self.title {
            builder = builder.with_title(title);
        }
        builder.build().map_err(|error| error.to_string())
    }

    fn native_icon(&self) -> Result<Option<Icon>, String> {
        match &self.icon {
            Change::Set((rgba, width, height)) => {
                let needed = u64::from(*width) * u64::from(*height) * 4;
                if u64::try_from(rgba.len()).ok() != Some(needed) {
                    return Err(format!(
                        "a {width}x{height} RGBA icon needs {needed} bytes, got {}",
                        rgba.len()
                    ));
                }
                Icon::from_rgba(rgba.clone(), *width, *height)
                    .map(Some)
                    .map_err(|error| error.to_string())
            }
            Change::Keep | Change::Clear => Ok(None),
        }
    }

    fn native_menu(&self) -> Result<Option<muda::Menu>, String> {
        match &self.menu {
            Change::Set(menu) => menu.native().map(Some),
            Change::Keep | Change::Clear => Ok(None),
        }
    }
}

/// A pointer event on the tray icon.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum TrayEvent {
    /// The click tray event.
    Click {
        /// The button.
        button: MouseButton,
        /// The position.
        position: Point,
    },
    /// The double click tray event.
    DoubleClick {
        /// The button.
        button: MouseButton,
        /// The position.
        position: Point,
    },
    /// The enter tray event.
    Enter,
    /// The leave tray event.
    Leave,
}

/// Delivers `event` as if the pointer caused it.
#[cfg(feature = "kittest")]
pub fn simulate(event: TrayEvent) {
    Tray::events().emit(event);
}

pub(crate) fn has_pending() -> bool {
    NATIVE.with(Native::has_pending)
}

pub(crate) fn install(wake: Wake) {
    if has_pending() {
        install_tray(wake);
    }
}

pub(crate) fn pump() {
    NATIVE.with(Native::pump);
}

fn install_tray(wake: Wake) {
    let built = NATIVE.with(|native| {
        native.install(|tray| {
            crate::app::menu::connect(std::sync::Arc::clone(&wake));
            tray.native()
        })
    });
    match built {
        Ok(true) => {}
        Ok(false) => return,
        Err(error) => {
            tracing::warn!(target: "sigil::tray", %error, "tray icon build failed");
            return;
        }
    }

    let sink = NATIVE.with(|native| native.sink(wake));
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let Some(event) = map_event(&event) {
            sink(event);
        }
    }));
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "screen coordinates fit an f32"
)]
const fn map_event(event: &TrayIconEvent) -> Option<TrayEvent> {
    match event {
        TrayIconEvent::Click {
            button,
            button_state: tray_icon::MouseButtonState::Up,
            position,
            ..
        } => Some(TrayEvent::Click {
            button: map_button(*button),
            position: Point::new(position.x as f32, position.y as f32),
        }),
        TrayIconEvent::DoubleClick {
            button, position, ..
        } => Some(TrayEvent::DoubleClick {
            button: map_button(*button),
            position: Point::new(position.x as f32, position.y as f32),
        }),
        TrayIconEvent::Enter { .. } => Some(TrayEvent::Enter),
        TrayIconEvent::Leave { .. } => Some(TrayEvent::Leave),
        _ => None,
    }
}

const fn map_button(button: NativeButton) -> MouseButton {
    match button {
        NativeButton::Left => MouseButton::Left,
        NativeButton::Right => MouseButton::Right,
        NativeButton::Middle => MouseButton::Middle,
    }
}

#[cfg(test)]
#[path = "tray_tests.rs"]
mod tests;
