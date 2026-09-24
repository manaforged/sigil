use crate::reactive::signal::Signal;

pub trait Reactive<T>: 'static {
    fn read(&self) -> T;
}

impl<T: Clone + 'static> Reactive<T> for Signal<T> {
    #[track_caller]
    fn read(&self) -> T {
        self.get()
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
