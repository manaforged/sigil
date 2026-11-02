use std::error::Error;

/// State of a task.
#[derive(Debug, Clone)]
pub enum Status<T, E = Box<dyn Error + Send + Sync>> {
    /// The idle status.
    Idle,
    /// The pending status.
    Pending,
    /// The ok status.
    Ok(T),
    /// The err status.
    Err(E),
    /// The panicked status.
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
