use crate::reactive::memo;
use crate::reactive::signal::{Signal, signal};

/// A value, signal, or closure that a widget reads reactively.
pub trait Reactive<T>: 'static {
    /// Reads the current value, subscribing the enclosing reactive scope when backed by a signal.
    fn read(&self) -> T;

    /// Returns the value when this input never changes, so a widget can skip the reactive binding.
    fn try_static(&self) -> Option<T> {
        None
    }

    /// Calls `f` with a reference to the current value, without a copy when backed by a signal.
    #[track_caller]
    fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R
    where
        Self: Sized,
    {
        f(&self.read())
    }

    /// Converts this reactive input into a [`Signal<T>`], returning an existing signal unchanged.
    #[must_use]
    fn into_signal(self) -> Signal<T>
    where
        Self: Sized,
        T: Clone + PartialEq + 'static,
    {
        memo(move || self.read())
    }
}

impl<T: Clone + 'static> Reactive<T> for Signal<T> {
    #[track_caller]
    fn read(&self) -> T {
        self.get()
    }

    #[track_caller]
    fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R
    where
        Self: Sized,
    {
        Self::with(*self, f)
    }

    fn into_signal(self) -> Self
    where
        T: PartialEq,
    {
        self
    }
}

impl Reactive<f32> for crate::animation::Animator {
    #[track_caller]
    fn read(&self) -> f32 {
        self.get()
    }

    fn into_signal(self) -> Signal<f32> {
        self.value()
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

macro_rules! static_reactive {
    (clone: $($ty:ty),+ $(,)?) => {
        $(static_reactive!(@impl $ty, value => value.clone());)+
    };
    (copy: $($ty:ty),+ $(,)?) => {
        $(static_reactive!(@impl $ty, value => *value);)+
    };
    (@impl $ty:ty, $value:ident => $read:expr) => {
        impl Reactive<Self> for $ty {
            fn read(&self) -> Self {
                let $value = self;
                $read
            }

            fn try_static(&self) -> Option<Self> {
                Some(Reactive::<Self>::read(self))
            }

            fn into_signal(self) -> Signal<Self> {
                signal(self)
            }
        }
    };
}

static_reactive!(clone: String, crate::style::Theme);

static_reactive!(
    copy: bool,
    f32,
    f64,
    usize,
    isize,
    u8,
    u16,
    u32,
    u64,
    i8,
    i16,
    i32,
    i64,
    char,
    crate::geometry::Point,
    crate::geometry::Rect,
    crate::geometry::Size,
    crate::style::Tone,
    crate::style::Color,
    crate::style::ColorValue,
    crate::style::Gradient,
    crate::style::Background,
    crate::style::ControlSize,
    crate::style::Layer,
    crate::style::Token,
    crate::widgets::Density,
    crate::widgets::DiffView,
    crate::widgets::Disclosure,
    crate::events::Cursor,
);

impl Reactive<String> for &'static str {
    fn read(&self) -> String {
        (*self).to_owned()
    }

    fn try_static(&self) -> Option<String> {
        Some(self.read())
    }

    fn into_signal(self) -> Signal<String> {
        signal(self.to_owned())
    }
}

impl<T: Clone + 'static> Reactive<Self> for Vec<T> {
    fn read(&self) -> Self {
        self.clone()
    }

    fn try_static(&self) -> Option<Self> {
        Some(self.clone())
    }

    fn with<R>(&self, f: impl FnOnce(&Self) -> R) -> R
    where
        Self: Sized,
    {
        f(self)
    }
}

impl<T: Clone + 'static, const N: usize> Reactive<Vec<T>> for [T; N] {
    fn read(&self) -> Vec<T> {
        self.to_vec()
    }

    fn try_static(&self) -> Option<Vec<T>> {
        Some(self.to_vec())
    }
}

impl<T: Clone + 'static> Reactive<Vec<T>> for &'static [T] {
    fn read(&self) -> Vec<T> {
        self.to_vec()
    }

    fn try_static(&self) -> Option<Vec<T>> {
        Some(self.to_vec())
    }
}

impl<T: Clone + PartialEq + 'static> Reactive<Self> for Option<T> {
    fn read(&self) -> Self {
        self.clone()
    }

    fn try_static(&self) -> Option<Self> {
        Some(self.clone())
    }

    fn into_signal(self) -> Signal<Self> {
        signal(self)
    }
}

impl Reactive<Option<String>> for Option<&'static str> {
    fn read(&self) -> Option<String> {
        self.map(str::to_owned)
    }

    fn try_static(&self) -> Option<Option<String>> {
        Some(self.read())
    }

    fn into_signal(self) -> Signal<Option<String>> {
        signal(self.map(str::to_owned))
    }
}
