use crate::reactive::derived;
use crate::reactive::signal::{Signal, signal};

pub trait Reactive<T>: 'static {
    fn read(&self) -> T;

    #[must_use]
    fn into_signal(self) -> Signal<T>
    where
        Self: Sized,
        T: Clone + PartialEq + 'static,
    {
        derived(move || self.read())
    }
}

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

impl<T, F> Reactive<T> for F
where
    F: Fn() -> T + 'static,
{
    fn read(&self) -> T {
        self()
    }
}

impl Reactive<Self> for bool {
    fn read(&self) -> Self {
        *self
    }

    fn into_signal(self) -> Signal<Self> {
        signal(self)
    }
}
