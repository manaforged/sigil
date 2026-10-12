use std::sync::Arc;

use crate::app::WindowCommand;
use crate::events::ShellEvent;
use crate::geometry::{LogicalSize, PhysicalSize, Rect};
use crate::hal::Shell;
use winit::dpi::{LogicalPosition as WinitLogicalPosition, LogicalSize as WinitLogicalSize};

use crate::app::WindowOpts;
use crate::shell::{ShellError, WinitWindowHandle, app::WinitApp, waker::WinitWaker};

use super::{PendingWindowSize, WinitShell};

impl Shell for WinitShell {
    type Error = ShellError;
    type Waker = WinitWaker;
    type Window = WinitWindowHandle;

    fn waker(&self) -> Self::Waker {
        WinitWaker {
            proxy: Arc::clone(&self.proxy),
            pending: Arc::clone(&self.pending_wake),
            alive: Arc::clone(&self.alive),
        }
    }

    fn wake(waker: &Self::Waker) -> Result<(), Self::Error> {
        waker.wake()
    }

    fn create_window(
        &mut self,
        title: &str,
        initial_size: PhysicalSize,
    ) -> Result<Self::Window, Self::Error> {
        Ok(self.queue_window(
            title,
            PendingWindowSize::Physical(initial_size),
            WindowOpts::default(),
        ))
    }

    fn create_window_logical(
        &mut self,
        title: &str,
        initial: LogicalSize,
    ) -> Result<Self::Window, Self::Error> {
        Ok(self.queue_window(
            title,
            PendingWindowSize::Logical(initial),
            WindowOpts::default(),
        ))
    }

    fn create_window_with(
        &mut self,
        title: &str,
        initial: LogicalSize,
        opts: WindowOpts,
    ) -> Result<Self::Window, Self::Error> {
        Ok(self.queue_window(title, PendingWindowSize::Logical(initial), opts))
    }

    fn drain_events(&mut self) -> std::vec::IntoIter<ShellEvent> {
        self.drain_tagged_events()
            .map(|(_, event)| event)
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn set_ime_allowed(&mut self, window: &Self::Window, allowed: bool) -> Result<(), Self::Error> {
        #[cfg(target_arch = "wasm32")]
        if let Some(ime) = &mut self.web_ime {
            ime.set_allowed(allowed);
        }
        if let Some(window) = self.window_for(*window) {
            window.set_ime_allowed(allowed);
        }
        Ok(())
    }

    fn set_ime_cursor_area(
        &mut self,
        window: &Self::Window,
        rect: Rect,
    ) -> Result<(), Self::Error> {
        let zoom = self.window_zoom(*window);
        #[cfg(target_arch = "wasm32")]
        if let Some(ime) = &self.web_ime {
            ime.set_cursor_area(rect, zoom);
        }
        if let Some(window) = self.window_for(*window) {
            let at = |value: f32| f64::from(value * zoom);
            window.set_ime_cursor_area(
                WinitLogicalPosition::new(at(rect.origin.x), at(rect.origin.y)),
                WinitLogicalSize::new(at(rect.width), at(rect.height)),
            );
        }
        Ok(())
    }

    fn window_command(
        &mut self,
        window: &Self::Window,
        command: WindowCommand,
    ) -> Result<(), Self::Error> {
        self.run_window_command(*window, command);
        Ok(())
    }

    fn run<F>(mut self, frame: F) -> Result<(), Self::Error>
    where
        F: FnMut(&mut Self) + 'static,
    {
        let event_loop = self.event_loop.take().ok_or(ShellError::AlreadyRun)?;
        let app = WinitApp {
            shell: self,
            frame,
            app_redraw_pending: false,
        };
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys as _;
            event_loop.spawn_app(app);
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut app = app;
            event_loop.run_app(&mut app)?;
            match app.shell.take_failed_windows().into_iter().next() {
                Some((_, error)) => Err(ShellError::Window(error)),
                None => Ok(()),
            }
        }
    }
}
