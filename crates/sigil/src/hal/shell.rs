use core::error::Error;

use alloc::vec::IntoIter;

use crate::hal::{
    ShellEvent,
    geometry::{PhysicalSize, Rect},
    size::{LogicalSize, Scale},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anchor {
    BottomCenter { margin: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WindowOpts {
    pub overlay: bool,
    pub frameless: bool,
    pub anchor: Option<Anchor>,
    pub user_zoom: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResizeDirection {
    East,
    North,
    NorthEast,
    NorthWest,
    South,
    SouthEast,
    SouthWest,
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowCommand {
    Close,
    Minimize,
    ToggleMaximize,
    Drag,
    DragResize(ResizeDirection),
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

    fn drain_events(&mut self) -> IntoIter<ShellEvent>;

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
