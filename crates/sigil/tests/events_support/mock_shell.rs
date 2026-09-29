#![allow(clippy::redundant_pub_crate, dead_code)]

use std::error::Error;
use std::fmt;

use sigil::events::ShellEvent;
use sigil::geometry::PhysicalSize;
use sigil::hal::Shell;

#[derive(Default)]
pub(crate) struct MockShell {
    events: Vec<ShellEvent>,
    pub(crate) created: Vec<(String, PhysicalSize)>,
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
    type Window = usize;

    fn waker(&self) -> Self::Waker {}

    fn wake(_waker: &Self::Waker) -> Result<(), Self::Error> {
        Ok(())
    }

    fn create_window(
        &mut self,
        title: &str,
        initial_size: PhysicalSize,
    ) -> Result<Self::Window, Self::Error> {
        self.created.push((title.to_owned(), initial_size));
        Ok(self.created.len() - 1)
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
