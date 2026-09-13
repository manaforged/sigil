#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use std::fmt;

use sigil::hal::{LogicalSize, PhysicalSize, Shell, ShellEvent};

#[derive(Debug)]
struct MockError;

impl fmt::Display for MockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("mock shell error")
    }
}

impl std::error::Error for MockError {}

#[derive(Clone)]
struct MockWaker;

#[derive(Default)]
struct MockShell {
    created: Vec<(String, PhysicalSize)>,
}

impl Shell for MockShell {
    type Error = MockError;
    type Waker = MockWaker;
    type Window = usize;

    fn waker(&self) -> Self::Waker {
        MockWaker
    }

    fn wake(_waker: &Self::Waker) -> Result<(), Self::Error> {
        Ok(())
    }

    fn create_window(
        &mut self,
        title: &str,
        initial_size: PhysicalSize,
    ) -> Result<Self::Window, Self::Error> {
        let handle = self.created.len();
        self.created.push((title.to_owned(), initial_size));
        Ok(handle)
    }

    fn drain_events(&mut self) -> std::vec::IntoIter<ShellEvent> {
        Vec::new().into_iter()
    }

    fn run<F>(self, _frame: F) -> Result<(), Self::Error>
    where
        F: FnMut(&mut Self) + 'static,
    {
        Ok(())
    }
}

#[test]
fn default_create_window_logical_uses_scale_one_physical_size() {
    let mut shell = MockShell::default();

    let handle = shell
        .create_window_logical("test", LogicalSize::new(720.0, 480.0))
        .expect("default logical create should delegate");

    assert_eq!(handle, 0);
    assert_eq!(
        shell.created,
        vec![("test".to_owned(), PhysicalSize::new(720, 480))]
    );
}

#[test]
fn default_create_window_with_ignores_opts_and_delegates() {
    let mut shell = MockShell::default();

    let handle = shell
        .create_window_with(
            "pill",
            LogicalSize::new(200.0, 48.0),
            sigil::hal::WindowOpts {
                overlay: true,
                frameless: false,
                anchor: Some(sigil::hal::Anchor::BottomCenter { margin: 64.0 }),
                user_zoom: None,
            },
        )
        .expect("default opts create should delegate");

    assert_eq!(handle, 0);
    assert_eq!(
        shell.created,
        vec![("pill".to_owned(), PhysicalSize::new(200, 48))]
    );
}
