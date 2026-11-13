use crate::reactive::signal::{Signal, signal};

pub(crate) mod sealed {
    use crate::reactive::memo;
    use crate::reactive::signal::Signal;

    #[allow(unnameable_types, reason = "sealed")]
    pub trait Sealed<T>: 'static {
        fn read_value(&self) -> T;

        #[must_use]
        fn to_signal(self) -> Signal<T>
        where
            Self: Sized,
            T: Clone + PartialEq + 'static,
        {
            memo(move || self.read_value())
        }
    }
}

/// A value, signal, or closure that a widget reads reactively.
pub trait Reactive<T>: sealed::Sealed<T> + 'static {
    /// Reads the current value, subscribing the enclosing reactive scope when backed by a signal.
    #[track_caller]
    fn read(&self) -> T {
        self.read_value()
    }

    /// Converts this reactive input into a [`Signal<T>`], returning an existing signal unchanged.
    #[must_use]
    fn into_signal(self) -> Signal<T>
    where
        Self: Sized,
        T: Clone + PartialEq + 'static,
    {
        self.to_signal()
    }
}

impl<T, R: sealed::Sealed<T> + 'static> Reactive<T> for R {}

impl<T: Clone + 'static> sealed::Sealed<T> for Signal<T> {
    #[track_caller]
    fn read_value(&self) -> T {
        self.get()
    }

    fn to_signal(self) -> Self
    where
        T: PartialEq,
    {
        self
    }
}

impl sealed::Sealed<f32> for crate::animation::Animator {
    #[track_caller]
    fn read_value(&self) -> f32 {
        self.get()
    }

    fn to_signal(self) -> Signal<f32> {
        self.value()
    }
}

impl<T, F> sealed::Sealed<T> for F
where
    F: Fn() -> T + 'static,
{
    fn read_value(&self) -> T {
        self()
    }
}

impl sealed::Sealed<Self> for String {
    fn read_value(&self) -> Self {
        self.clone()
    }

    fn to_signal(self) -> Signal<Self> {
        signal(self)
    }
}

impl sealed::Sealed<String> for &'static str {
    fn read_value(&self) -> String {
        (*self).to_owned()
    }

    fn to_signal(self) -> Signal<String> {
        signal(self.to_owned())
    }
}

impl<T: Clone + 'static> sealed::Sealed<Self> for Vec<T> {
    fn read_value(&self) -> Self {
        self.clone()
    }
}

impl<T: Clone + 'static, const N: usize> sealed::Sealed<Vec<T>> for [T; N] {
    fn read_value(&self) -> Vec<T> {
        self.to_vec()
    }
}

macro_rules! copy_reactive {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl sealed::Sealed<Self> for $ty {
                fn read_value(&self) -> Self {
                    *self
                }

                fn to_signal(self) -> Signal<Self> {
                    signal(self)
                }
            }
        )+
    };
}

copy_reactive!(
    bool,
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
    crate::style::ControlSize,
    crate::style::Layer,
    crate::style::Token,
    crate::widgets::Density,
    crate::widgets::DiffView,
    crate::widgets::Disclosure,
);

impl<T: Clone + 'static> sealed::Sealed<Vec<T>> for &'static [T] {
    fn read_value(&self) -> Vec<T> {
        self.to_vec()
    }
}

impl<T: Clone + PartialEq + 'static> sealed::Sealed<Self> for Option<T> {
    fn read_value(&self) -> Self {
        self.clone()
    }

    fn to_signal(self) -> Signal<Self> {
        signal(self)
    }
}

impl sealed::Sealed<Option<String>> for Option<&'static str> {
    fn read_value(&self) -> Option<String> {
        self.map(str::to_owned)
    }

    fn to_signal(self) -> Signal<Option<String>> {
        signal(self.map(str::to_owned))
    }
}
