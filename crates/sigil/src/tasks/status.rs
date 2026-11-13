/// State of a task.
#[derive(Debug, Clone)]
pub enum Status<T, E = crate::Error> {
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
    /// Returns whether this status is idle.
    #[must_use]
    pub const fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

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

    /// Converts `Status<T, E>` to `Option<T>`, discarding any non-`Ok` state.
    #[must_use]
    pub fn ok(self) -> Option<T> {
        if let Self::Ok(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Converts `Status<T, E>` to `Option<E>`, discarding any non-`Err` state.
    #[must_use]
    pub fn err(self) -> Option<E> {
        if let Self::Err(e) = self {
            Some(e)
        } else {
            None
        }
    }

    /// Converts from `&Status<T, E>` to `Status<&T, &E>`.
    #[must_use]
    pub fn as_ref(&self) -> Status<&T, &E> {
        match self {
            Self::Idle => Status::Idle,
            Self::Pending => Status::Pending,
            Self::Ok(v) => Status::Ok(v),
            Self::Err(e) => Status::Err(e),
            Self::Panicked(msg) => Status::Panicked(msg.clone()),
        }
    }

    /// Maps a `Status<T, E>` to `Status<U, E>` by applying `f` to an `Ok` value.
    #[must_use]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Status<U, E> {
        match self {
            Self::Idle => Status::Idle,
            Self::Pending => Status::Pending,
            Self::Ok(v) => Status::Ok(f(v)),
            Self::Err(e) => Status::Err(e),
            Self::Panicked(msg) => Status::Panicked(msg),
        }
    }
}
