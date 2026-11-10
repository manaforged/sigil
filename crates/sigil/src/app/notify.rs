use crate::error::{AppResult, Error};

/// The content of a desktop notification.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct Notification {
    /// The title.
    pub title: String,
    /// The body.
    pub body: String,
    /// The app name, if any.
    pub app_name: Option<String>,
}

impl Notification {
    /// Creates a notification.
    #[must_use]
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            app_name: None,
        }
    }

    /// Sets the app name.
    #[must_use]
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    /// Shows the notification on the desktop.
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        all(unix, not(target_os = "android"))
    ))]
    pub fn send(&self) -> AppResult<()> {
        let mut native = notify_rust::Notification::new();
        native.summary(&self.title).body(&self.body);
        if let Some(name) = &self.app_name {
            native.appname(name);
        }
        native.show().map(|_| ()).map_err(Error::new)
    }

    /// Returns an unsupported error on this target.
    #[cfg(not(any(
        target_os = "macos",
        target_os = "windows",
        all(unix, not(target_os = "android"))
    )))]
    pub fn send(&self) -> AppResult<()> {
        Err(Error::new(std::io::Error::from(
            std::io::ErrorKind::Unsupported,
        )))
    }
}
