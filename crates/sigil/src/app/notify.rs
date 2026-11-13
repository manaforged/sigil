use crate::error::{AppResult, Error};

/// The content of a desktop notification.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct Notification {
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) app_name: Option<String>,
}

impl Notification {
    /// Creates a notification with a title and body text.
    #[must_use]
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            app_name: None,
        }
    }

    /// Sets the app name that is sent with the notification.
    #[must_use]
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    /// Shows the notification on the desktop.
    #[cfg(os_notifications)]
    pub fn send(&self) -> AppResult<()> {
        let mut native = notify_rust::Notification::new();
        native.summary(&self.title).body(&self.body);
        if let Some(name) = &self.app_name {
            native.appname(name);
        }
        native
            .show()
            .map(|_| ())
            .map_err(|err| Error::new(crate::ErrorKind::Other, err))
    }

    /// Returns an unsupported error on this target.
    #[cfg(not(os_notifications))]
    pub fn send(&self) -> AppResult<()> {
        Err(Error::new(
            crate::ErrorKind::Other,
            std::io::Error::from(std::io::ErrorKind::Unsupported),
        ))
    }
}
