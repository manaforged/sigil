#![allow(clippy::redundant_pub_crate)]

use std::error::Error;
use std::fmt;

use sigil_hal::{PhysicalSize, Shell, ShellEvent};

#[derive(Default)]
pub(crate) struct MockShell {
    events: Vec<ShellEvent>,
    next_window: u64,
}

impl MockShell {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push_event(&mut self, event: ShellEvent) {
        self.events.push(event);
    }
}

#[derive(Debug)]
pub(crate) struct MockShellError;

impl fmt::Display for MockShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("mock shell error")
    }
}

impl Error for MockShellError {}

impl Shell for MockShell {
    type Error = MockShellError;
    type Waker = ();
    type Window = u64;

    fn waker(&self) -> Self::Waker {}

    fn wake(_waker: &Self::Waker) -> Result<(), Self::Error> {
        Ok(())
    }

    fn create_window(
        &mut self,
        _title: &str,
        _initial_size: PhysicalSize,
    ) -> Result<Self::Window, Self::Error> {
        let id = self.next_window;
        self.next_window = self.next_window.wrapping_add(1);
        Ok(id)
    }

    fn drain_events(&mut self) -> std::vec::IntoIter<ShellEvent> {
        std::mem::take(&mut self.events).into_iter()
    }

    fn run<F>(mut self, mut frame: F) -> Result<(), Self::Error>
    where
        F: FnMut(&mut Self) + 'static,
    {
        frame(&mut self);
        Ok(())
    }
}
