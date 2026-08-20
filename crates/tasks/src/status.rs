//! [`Status`] — the three-state result signal carried by
//! [`spawn_to_signal`][crate::spawn_to_signal].
//!
//! See DR-007. `Signal<Status<T, E>>` lets components observe a
//! background future's lifecycle the same way they observe any other
//! reactive value.

use std::error::Error;

/// Lifecycle of a value produced by a background future.
///
/// Carried by `Signal<Status<T, E>>` from
/// [`spawn_to_signal`][crate::spawn_to_signal]. Starts as
/// [`Status::Pending`]; on future completion the main-thread
/// dispatcher writes [`Status::Ok`] or [`Status::Err`] into the
/// signal. Components that read the signal re-run automatically on
/// every transition. See DR-007.
///
/// `E` defaults to a boxed trait object so the common case
/// (`spawn_to_signal(owner, async { ... })`) is single-generic at
/// the call site.
#[derive(Debug)]
pub enum Status<T, E = Box<dyn Error + Send + Sync>> {
    /// Future has not yet completed.
    Pending,
    /// Future completed with `Ok(T)`.
    Ok(T),
    /// Future completed with `Err(E)` — or was cancelled by owner
    /// disposal, in which case the signal stays at `Pending` and the
    /// task is dropped without writing back. `Err` is only ever
    /// written for a real error returned by the user future.
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
    /// `true` iff the future has not completed yet.
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// `true` iff the future completed with `Ok`.
    #[must_use]
    pub const fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    /// `true` iff the future completed with `Err`.
    #[must_use]
    pub const fn is_err(&self) -> bool {
        matches!(self, Self::Err(_))
    }
}
