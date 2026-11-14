use core::error::Error;

use alloc::vec::IntoIter;

use crate::hal::cursor::CursorIcon;
use crate::hal::events::Event;
use crate::hal::{
    geometry::{PhysicalSize, Point, Rect},
    size::{Scale, Size},
};

/// Screen anchor of an overlay window.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Anchor {
    /// Centres the window horizontally at the bottom of the monitor.
    BottomCenter {
        /// The gap between the window and the bottom edge of the monitor, in logical pixels.
        margin: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WindowOpts {
    pub overlay: bool,
    pub frameless: bool,
    pub inset_title_bar: Option<f32>,
    pub anchor: Option<Anchor>,
    pub user_zoom: Option<f32>,
    pub min_size: Option<Size>,
    pub fixed_size: bool,
}

pub use crate::hal::cursor::ResizeDirection;

/// Window stacking level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum WindowLevel {
    /// The window stays below all other windows.
    AlwaysOnBottom,
    /// The window stacks with other windows in the usual way.
    #[default]
    Normal,
    /// The window stays above all other windows.
    AlwaysOnTop,
}

/// Commands for the window whose frame runs, or for a `Window` through `Window::command`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WindowCommand {
    /// Asks the window to close; the app receives `WindowEvent::CloseRequested`.
    Close,
    /// Minimizes the window.
    Minimize,
    /// Maximizes the window, or restores it when it is maximized.
    ToggleMaximize,
    /// Starts a move of the window that follows the pointer.
    Drag,
    /// Starts a resize of the window from an edge or a corner that follows the pointer.
    DragResize(ResizeDirection),
    /// Sets the zoom of the window in percent, where 100 is no zoom; 0 does nothing.
    Zoom(u16),
    /// Makes the window visible.
    Show,
    /// Hides the window.
    Hide,
    /// Makes the window visible and gives it input focus.
    Focus,
    /// Sets the title of the window.
    Title(alloc::string::String),
    /// Moves the outer frame of the window to a position in logical pixels.
    Position(Point),
    /// Sets the stacking level of the window.
    Level(WindowLevel),
}

pub trait Shell: Sized {
    type Error: Error + Send + Sync + 'static;

    type Waker: Clone + Send + Sync + 'static;

    type Window;

    fn waker(&self) -> Self::Waker;

    fn wake(waker: &Self::Waker) -> Result<(), Self::Error>;

    fn create_window(
        &mut self,
        title: &str,
        initial_size: PhysicalSize,
    ) -> Result<Self::Window, Self::Error>;

    fn create_window_logical(
        &mut self,
        title: &str,
        initial: Size,
    ) -> Result<Self::Window, Self::Error> {
        self.create_window(title, Scale::ONE.to_physical(initial))
    }

    fn create_window_with(
        &mut self,
        title: &str,
        initial: Size,
        opts: WindowOpts,
    ) -> Result<Self::Window, Self::Error> {
        let _ = opts;
        self.create_window_logical(title, initial)
    }

    #[cfg(feature = "kittest")]
    fn drain_events(&mut self) -> IntoIter<Event>;

    fn run<F>(self, frame: F) -> Result<(), Self::Error>
    where
        F: FnMut(&mut Self) + 'static;

    fn set_ime_allowed(&mut self, window: &Self::Window, allowed: bool) -> Result<(), Self::Error> {
        let _ = (window, allowed);
        Ok(())
    }

    fn set_ime_cursor_area(
        &mut self,
        window: &Self::Window,
        rect: Rect,
    ) -> Result<(), Self::Error> {
        let _ = (window, rect);
        Ok(())
    }

    fn set_cursor(&mut self, window: &Self::Window, cursor: CursorIcon) -> Result<(), Self::Error> {
        let _ = (window, cursor);
        Ok(())
    }

    fn window_command(
        &mut self,
        window: &Self::Window,
        command: WindowCommand,
    ) -> Result<(), Self::Error> {
        let _ = (window, command);
        Ok(())
    }
}
