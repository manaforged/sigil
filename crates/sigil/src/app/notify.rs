use crate::app::error::{AppResult, Error};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct Notification {
    pub title: String,
    pub body: String,
    pub app_name: Option<String>,
}

impl Notification {
    #[must_use]
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            app_name: None,
        }
    }

    #[must_use]
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    all(unix, not(target_os = "android"))
))]
pub fn send(notification: &Notification) -> AppResult<()> {
    let mut native = notify_rust::Notification::new();
    native.summary(&notification.title).body(&notification.body);
    if let Some(name) = &notification.app_name {
        native.appname(name);
    }
    native.show().map(|_| ()).map_err(Error::new)
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "windows",
    all(unix, not(target_os = "android"))
)))]
pub fn send(_notification: &Notification) -> AppResult<()> {
    Err(Error::new(std::io::Error::from(
        std::io::ErrorKind::Unsupported,
    )))
}
