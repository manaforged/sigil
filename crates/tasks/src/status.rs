use std::error::Error;

#[derive(Debug)]
pub enum Status<T, E = Box<dyn Error + Send + Sync>> {
    Pending,
    Ok(T),
    Err(E),
}

impl<T: Clone, E: Clone> Clone for Status<T, E> {
    fn clone(&self) -> Self {
        match self {
            Self::Pending => Self::Pending,
            Self::Ok(v) => Self::Ok(v.clone()),
            Self::Err(e) => Self::Err(e.clone()),
        }
    }
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
}
