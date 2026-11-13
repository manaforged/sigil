use std::error::Error;

/// State of a task.
#[derive(Debug, Clone)]
pub enum Status<T, E = Box<dyn Error + Send + Sync>> {
    /// The task is not running: it has not started, or it was cancelled.
    Idle,
    /// The task is running and has no result yet.
    Pending,
    /// The task finished and returned this value.
    Ok(T),
    /// The task finished and returned this error.
    Err(E),
    /// The task panicked; the string is the panic message.
    Panicked(String),
}

impl<T, E> Status<T, E> {
    /// Returns whether this status is pending.
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns whether this status is ok.
    #[must_use]
    pub const fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    /// Returns whether this status is err.
    #[must_use]
    pub const fn is_err(&self) -> bool {
        matches!(self, Self::Err(_))
    }

    /// Returns whether this status is panicked.
    #[must_use]
    pub const fn is_panicked(&self) -> bool {
        matches!(self, Self::Panicked(_))
    }
}
