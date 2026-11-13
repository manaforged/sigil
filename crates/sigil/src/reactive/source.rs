use crate::reactive::signal::{Signal, signal};

pub(crate) mod sealed {
    use crate::reactive::memo;
    use crate::reactive::signal::Signal;

    #[allow(unnameable_types, reason = "sealed")]
    pub trait Sealed<T>: 'static {
        fn read(&self) -> T;

        #[must_use]
        fn into_signal(self) -> Signal<T>
        where
            Self: Sized,
            T: Clone + PartialEq + 'static,
        {
            memo(move || self.read())
        }
    }
}

/// A value or a closure that a widget reads inside an effect.
pub trait Reactive<T>: sealed::Sealed<T> + 'static {}

impl<T, R: sealed::Sealed<T> + 'static> Reactive<T> for R {}

impl<T: Clone + 'static> sealed::Sealed<T> for Signal<T> {
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

impl<T, F> sealed::Sealed<T> for F
where
    F: Fn() -> T + 'static,
{
    fn read(&self) -> T {
        self()
    }
}

impl sealed::Sealed<Self> for String {
    fn read(&self) -> Self {
        self.clone()
    }

    fn into_signal(self) -> Signal<Self> {
        signal(self)
    }
}

impl sealed::Sealed<String> for &'static str {
    fn read(&self) -> String {
        (*self).to_owned()
    }

    fn into_signal(self) -> Signal<String> {
        signal(self.to_owned())
    }
}

impl<T: Clone + 'static> sealed::Sealed<Self> for Vec<T> {
    fn read(&self) -> Self {
        self.clone()
    }
}

impl<T: Clone + 'static, const N: usize> sealed::Sealed<Vec<T>> for [T; N] {
    fn read(&self) -> Vec<T> {
        self.to_vec()
    }
}

impl sealed::Sealed<Self> for bool {
    fn read(&self) -> Self {
        *self
    }

    fn into_signal(self) -> Signal<Self> {
        signal(self)
    }
}

impl sealed::Sealed<Self> for f32 {
    fn read(&self) -> Self {
        *self
    }

    fn into_signal(self) -> Signal<Self> {
        signal(self)
    }
}
