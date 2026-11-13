use crate::reactive::memo;
use crate::reactive::signal::{Signal, signal};

pub(crate) mod sealed {
    #[allow(unnameable_types, reason = "sealed")]
    pub trait Sealed {}
}

/// A value or a closure that a widget reads inside an effect.
pub trait Reactive<T>: sealed::Sealed + 'static {
    /// The current value, read inside an effect so that a signal read subscribes.
    fn read(&self) -> T;

    /// Converts this reactive to signal.
    #[must_use]
    fn into_signal(self) -> Signal<T>
    where
        Self: Sized,
        T: Clone + PartialEq + 'static,
    {
        memo(move || self.read())
    }
}

impl<T: 'static> sealed::Sealed for Signal<T> {}
impl<T: Clone + 'static> Reactive<T> for Signal<T> {
    #[track_caller]
    fn read(&self) -> T {
        self.get()
    }

    fn into_signal(self) -> Self
    where
        T: PartialEq,
    {
        self
    }
}

impl<T, F: Fn() -> T + 'static> sealed::Sealed for F {}
impl<T, F> Reactive<T> for F
where
    F: Fn() -> T + 'static,
{
    fn read(&self) -> T {
        self()
    }
}

impl sealed::Sealed for String {}
impl Reactive<Self> for String {
    fn read(&self) -> Self {
        self.clone()
    }

    fn into_signal(self) -> Signal<Self> {
        signal(self)
    }
}

impl sealed::Sealed for &'static str {}
impl Reactive<String> for &'static str {
    fn read(&self) -> String {
        (*self).to_owned()
    }

    fn into_signal(self) -> Signal<String> {
        signal(self.to_owned())
    }
}

impl<T: 'static> sealed::Sealed for Vec<T> {}
impl<T: Clone + 'static> Reactive<Self> for Vec<T> {
    fn read(&self) -> Self {
        self.clone()
    }
}

impl<T: 'static, const N: usize> sealed::Sealed for [T; N] {}
impl<T: Clone + 'static, const N: usize> Reactive<Vec<T>> for [T; N] {
    fn read(&self) -> Vec<T> {
        self.to_vec()
    }
}

impl sealed::Sealed for bool {}
impl Reactive<Self> for bool {
    fn read(&self) -> Self {
        *self
    }

    fn into_signal(self) -> Signal<Self> {
        signal(self)
    }
}

impl sealed::Sealed for f32 {}
impl Reactive<Self> for f32 {
    fn read(&self) -> Self {
        *self
    }

    fn into_signal(self) -> Signal<Self> {
        signal(self)
    }
}
