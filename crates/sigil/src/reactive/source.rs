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

impl Reactive<Self> for String {
    fn read(&self) -> Self {
        self.clone()
    }

    fn into_signal(self) -> Signal<Self> {
        signal(self)
    }
}

impl Reactive<String> for &'static str {
    fn read(&self) -> String {
        (*self).to_owned()
    }

    fn into_signal(self) -> Signal<String> {
        signal(self.to_owned())
    }
}

impl<T: Clone + 'static> Reactive<Self> for Vec<T> {
    fn read(&self) -> Self {
        self.clone()
    }
}

impl<T: Clone + 'static, const N: usize> Reactive<Vec<T>> for [T; N] {
    fn read(&self) -> Vec<T> {
        self.to_vec()
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
