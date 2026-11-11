use core::error::Error;

use alloc::vec::IntoIter;

use crate::hal::events::Event;
use crate::hal::{
    geometry::{PhysicalSize, Point, Rect},
    size::{LogicalSize, Scale},
};

/// Screen anchor of an overlay window.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Anchor {
    /// The bottom center anchor.
    BottomCenter {
        /// The distance from the bottom edge.
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
    pub min_size: Option<LogicalSize>,
    pub fixed_size: bool,
}

/// Edge or corner for `WindowCommand::DragResize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResizeDirection {
    /// The east resize direction.
    East,
    /// The north resize direction.
    North,
    /// The north east resize direction.
    NorthEast,
    /// The north west resize direction.
    NorthWest,
    /// The south resize direction.
    South,
    /// The south east resize direction.
    SouthEast,
    /// The south west resize direction.
    SouthWest,
    /// The west resize direction.
    West,
}

/// Window stacking level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum WindowLevel {
    /// The always on bottom window level.
    AlwaysOnBottom,
    /// The normal window level.
    #[default]
    Normal,
    /// The always on top window level.
    AlwaysOnTop,
}

/// Commands for the window whose frame runs, or for a `WindowId` through `WindowId::command`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WindowCommand {
    /// The close window command.
    Close,
    /// The minimize window command.
    Minimize,
    /// The toggle maximize window command.
    ToggleMaximize,
    /// The drag window command.
    Drag,
    /// The drag resize window command.
    DragResize(ResizeDirection),
    /// The set zoom window command.
    SetZoom(u16),
    /// The show window command.
    Show,
    /// The hide window command.
    Hide,
    /// The focus window command.
    Focus,
    /// The set title window command.
    SetTitle(alloc::string::String),
    /// The set position window command.
    SetPosition(Point),
    /// The set level window command.
    SetLevel(WindowLevel),
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
        initial: LogicalSize,
    ) -> Result<Self::Window, Self::Error> {
        self.create_window(title, Scale::ONE.to_physical(initial))
    }

    fn create_window_with(
        &mut self,
        title: &str,
        initial: LogicalSize,
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

    fn window_command(
        &mut self,
        window: &Self::Window,
        command: WindowCommand,
    ) -> Result<(), Self::Error> {
        let _ = (window, command);
        Ok(())
    }
}
