use std::error::Error;

#[derive(Debug, Clone)]
pub enum Status<T, E = Box<dyn Error + Send + Sync>> {
    Idle,
    Pending,
    Ok(T),
    Err(E),
    Panicked(String),
}

impl<T, E> Status<T, E> {
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    #[must_use]
    pub const fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    #[must_use]
    pub const fn is_err(&self) -> bool {
        matches!(self, Self::Err(_))
    }

    #[must_use]
    pub const fn is_panicked(&self) -> bool {
        matches!(self, Self::Panicked(_))
    }
}
